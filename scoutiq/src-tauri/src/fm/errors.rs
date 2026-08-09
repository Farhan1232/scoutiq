//! German FM analysis error messages — specific, never generic cloud blame.

use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FmError {
    pub message: String,
}

impl FmError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn cancelled() -> Self {
        Self::msg("Analyse wurde abgebrochen.")
    }

    pub fn not_found() -> Self {
        Self::msg("FM-Spielstand konnte nicht geöffnet werden.")
    }

    pub fn permission() -> Self {
        Self::msg("FM-Spielstand konnte nicht geöffnet werden.")
    }

    pub fn open_failed() -> Self {
        Self::msg("FM-Spielstand konnte nicht geöffnet werden.")
    }

    pub fn unsupported() -> Self {
        Self::msg("Nicht unterstütztes Dateiformat.")
    }

    pub fn container() -> Self {
        Self::msg("Containerstruktur konnte nicht gelesen werden.")
    }

    pub fn decompress() -> Self {
        Self::msg("Komprimierter Datenblock konnte nicht entpackt werden.")
    }

    pub fn corrupted() -> Self {
        Self::msg("Spielstand scheint beschädigt zu sein.")
    }

    pub fn unknown() -> Self {
        Self::msg("Unbekannter Parserfehler.")
    }

    /// Only when path is clearly a cloud location AND content is not readable locally.
    pub fn cloud_unavailable(path: &Path) -> Self {
        let s = path.to_string_lossy();
        if s.contains("OneDrive") {
            Self::msg(
                "Datei lokal nicht verfügbar (OneDrive). Rechtsklick → „Immer auf diesem Gerät behalten“, dann erneut öffnen.",
            )
        } else if s.contains("Mobile Documents") || s.contains("CloudDocs") || s.contains("iCloud")
        {
            Self::msg(
                "Datei lokal nicht verfügbar (iCloud). Bitte Datei herunterladen oder lokal speichern.",
            )
        } else if s.contains("CloudStorage") || s.contains("Dropbox") || s.contains("Google Drive")
        {
            Self::msg(
                "Datei lokal nicht verfügbar (Cloud-Speicher). Bitte lokal verfügbar machen und erneut öffnen.",
            )
        } else {
            Self::msg("FM-Spielstand konnte nicht geöffnet werden.")
        }
    }
}

impl From<FmError> for String {
    fn from(value: FmError) -> Self {
        value.message
    }
}

pub fn looks_like_cloud_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("CloudStorage")
        || s.contains("OneDrive")
        || s.contains("Mobile Documents")
        || s.contains("CloudDocs")
        || s.contains("iCloud")
        || s.contains("Dropbox")
        || s.contains("/Google Drive/")
        || s.contains("GoogleDrive")
}

pub fn map_io_error(err: &io::Error, path: &Path) -> FmError {
    match err.kind() {
        io::ErrorKind::NotFound => {
            if looks_like_cloud_path(path) {
                FmError::cloud_unavailable(path)
            } else {
                FmError::not_found()
            }
        }
        io::ErrorKind::PermissionDenied => FmError::permission(),
        io::ErrorKind::InvalidData => FmError::corrupted(),
        _ => {
            let msg = format!("{err}").to_ascii_lowercase();
            let os_unavailable = err.raw_os_error() == Some(2)
                || msg.contains("unavailable")
                || msg.contains("icloud")
                || msg.contains("cloud");

            if looks_like_cloud_path(path) && os_unavailable {
                FmError::cloud_unavailable(path)
            } else {
                FmError::open_failed()
            }
        }
    }
}
