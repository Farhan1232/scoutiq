//! Read-only streamed FM file reader. Never uses read_to_end on the whole file.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::header::parse_fmf_header;
use crate::fm::errors::{map_io_error, FmError};
use crate::fm::reader::{bytes_to_hex, file_name_from_path, is_supported_extension};
use crate::fm::types::{ContainerType, FmFileInfo};

pub const HEADER_LEN: usize = 512;
const BUF_CAP: usize = 256 * 1024;

pub struct FmStreamReader {
    pub path: std::path::PathBuf,
    pub file_size: u64,
    reader: BufReader<File>,
}

impl FmStreamReader {
    pub fn open(path: &Path) -> Result<Self, FmError> {
        if !is_supported_extension(path) {
            return Err(FmError::unsupported());
        }
        let file = File::open(path).map_err(|e| map_io_error(&e, path))?;
        let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            path: path.to_path_buf(),
            file_size,
            reader: BufReader::with_capacity(BUF_CAP, file),
        })
    }

    pub fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, FmError> {
        self.reader
            .read(buf)
            .map_err(|e| map_io_error(&e, &self.path))
    }
}

pub fn open_file_info(path_str: &str) -> Result<FmFileInfo, FmError> {
    let path = Path::new(path_str);
    if !is_supported_extension(path) {
        return Err(FmError::unsupported());
    }
    let mut file = File::open(path).map_err(|e| map_io_error(&e, path))?;
    let meta = file.metadata().map_err(|e| map_io_error(&e, path))?;
    let file_size = meta.len();
    let modified_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));

    let mut header = vec![0u8; HEADER_LEN.min(file_size as usize)];
    let n = file.read(&mut header).map_err(|e| map_io_error(&e, path))?;
    header.truncate(n);

    let detected_container = detect_container(&header);

    Ok(FmFileInfo {
        file_name: file_name_from_path(path),
        file_path: path_str.to_string(),
        file_size,
        header_hex: bytes_to_hex(&header),
        detected_container,
        modified_unix,
    })
}

fn detect_container(header: &[u8]) -> Option<ContainerType> {
    Some(parse_fmf_header(header).container)
}

/// Read a bounded slice at offset (read-only).
pub fn read_slice(path: &Path, offset: u64, max_len: usize) -> Result<Vec<u8>, FmError> {
    let mut file = File::open(path).map_err(|e| map_io_error(&e, path))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| map_io_error(&e, path))?;
    let mut buf = vec![0u8; max_len];
    let n = file.read(&mut buf).map_err(|e| map_io_error(&e, path))?;
    buf.truncate(n);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_fmf_in_header() {
        let mut h = vec![0u8; 64];
        h[0] = 0x02;
        h[1] = 0x01;
        h[2..6].copy_from_slice(b"fmf.");
        h[26..30].copy_from_slice(&[0x28, 0xB5, 0x2F, 0xFD]);
        assert_eq!(detect_container(&h), Some(ContainerType::Fmf));
    }

    #[test]
    fn streaming_read_is_bounded() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "scoutiq_stream_{}.fm",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"FMF").unwrap();
            f.write_all(&vec![0u8; 10_000]).unwrap();
        }
        let info = open_file_info(path.to_str().unwrap()).expect("info");
        assert_eq!(info.file_size, 10_003);
        assert!(info.header_hex.len() < 10_000);
        assert_eq!(info.detected_container, Some(ContainerType::Fmf));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_unsupported_extension() {
        let err = open_file_info("/tmp/foo.txt").unwrap_err();
        assert!(err.message.contains("Dateiformat") || err.message.contains("unterstützt"));
    }
}
