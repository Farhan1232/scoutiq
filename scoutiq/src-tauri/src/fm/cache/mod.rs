//! Lightweight parser metadata cache (no decompressed payloads).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::fm::types::FmEngineResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheKey {
    pub path: String,
    pub file_size: u64,
    pub modified_unix: u64,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    key: CacheKey,
    result: FmEngineResult,
    stored_unix: u64,
}

pub struct ParserCache {
    inner: Mutex<HashMap<String, CacheEntry>>,
    disk_path: Option<PathBuf>,
}

impl ParserCache {
    pub fn memory_only() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            disk_path: None,
        }
    }

    pub fn with_disk(path: PathBuf) -> Self {
        let cache = Self {
            inner: Mutex::new(HashMap::new()),
            disk_path: Some(path.clone()),
        };
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(map) = serde_json::from_slice::<HashMap<String, CacheEntry>>(&bytes) {
                if let Ok(mut guard) = cache.inner.lock() {
                    *guard = map;
                }
            }
        }
        cache
    }

    pub fn get(&self, key: &CacheKey) -> Option<FmEngineResult> {
        let guard = self.inner.lock().ok()?;
        let entry = guard.get(&key.path)?;
        if entry.key.file_size == key.file_size
            && entry.key.modified_unix == key.modified_unix
            && entry.key.fingerprint == key.fingerprint
        {
            let mut r = entry.result.clone();
            r.from_cache = true;
            Some(r)
        } else {
            None
        }
    }

    pub fn put(&self, key: CacheKey, mut result: FmEngineResult) {
        result.from_cache = false;
        let entry = CacheEntry {
            key: key.clone(),
            result,
            stored_unix: now_unix(),
        };
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(key.path.clone(), entry);
            if let Some(path) = &self.disk_path {
                let _ = fs::write(path, serde_json::to_vec(&*guard).unwrap_or_default());
            }
        }
    }

    pub fn invalidate(&self, path: &str) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(path);
        }
    }
}

pub fn build_cache_key(
    path: &str,
    file_size: u64,
    modified_unix: Option<u64>,
    header_hex: &str,
) -> CacheKey {
    use crate::fm::entities::{PARSER_ENGINE_VERSION, PARSER_SCHEMA_VERSION};
    let fp = crate::fm::frames::types::fnv1a_64(
        format!("{header_hex}|{PARSER_ENGINE_VERSION}|{PARSER_SCHEMA_VERSION}").as_bytes(),
    );
    CacheKey {
        path: path.to_string(),
        file_size,
        modified_unix: modified_unix.unwrap_or(0),
        fingerprint: format!("{fp:016x}"),
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Global in-process cache for the app session.
pub fn global_cache() -> &'static ParserCache {
    use std::sync::OnceLock;
    static CACHE: OnceLock<ParserCache> = OnceLock::new();
    CACHE.get_or_init(|| {
        let path = std::env::temp_dir().join("scoutiq_fm_parser_cache.json");
        ParserCache::with_disk(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::types::{
        BlockStats, FmFileInfo, PerformanceDiagnostics, StructureStats, ANALYSIS_CALIBRATING,
        PARSER_STATUS,
    };

    fn dummy_result(path: &str) -> FmEngineResult {
        FmEngineResult {
            file: FmFileInfo {
                file_name: "t.fm".into(),
                file_path: path.into(),
                file_size: 10,
                header_hex: "00".into(),
                detected_container: None,
                modified_unix: Some(1),
            },
            blocks: vec![],
            block_stats: BlockStats {
                candidates: 0,
                validated: 0,
                decompressed: 0,
                errors: 0,
            },
            structures: vec![],
            structure_stats: StructureStats {
                string_tables: 0,
                record_tables: 0,
                index_tables: 0,
                internal_files: 0,
                unknown: 0,
            },
            string_index: vec![],
            string_entries: 0,
            performance: PerformanceDiagnostics {
                elapsed_ms: 1,
                bytes_processed: 10,
                decompressed_bytes: 0,
                blocks_processed: 0,
                mb_per_s: 0.0,
                cache_hit: false,
                errors: 0,
            },
            parser_status: PARSER_STATUS.into(),
            analysis_status: ANALYSIS_CALIBRATING.into(),
            notes: vec![],
            from_cache: false,
        }
    }

    #[test]
    fn invalidates_when_size_changes() {
        let cache = ParserCache::memory_only();
        let key = build_cache_key("/tmp/a.fm", 10, Some(1), "AA");
        cache.put(key.clone(), dummy_result("/tmp/a.fm"));
        assert!(cache.get(&key).is_some());
        let changed = build_cache_key("/tmp/a.fm", 11, Some(1), "AA");
        assert!(cache.get(&changed).is_none());
    }
}
