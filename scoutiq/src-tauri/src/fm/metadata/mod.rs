//! Metadata layer — calibrating only until deterministic records exist.
//!
//! Automatic field extraction is DISABLED. UI must show
//! "Parser wird kalibriert" instead of guessed values.

use serde::Serialize;

use crate::fm::container_types::CalibratingMetadata;

pub use crate::fm::container_types::CalibratingMetadata as MetadataState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPanel {
    pub calibrating: bool,
    pub message: String,
    pub fields: CalibratingMetadata,
}

pub fn calibrating_panel(save_name: Option<String>) -> MetadataPanel {
    MetadataPanel {
        calibrating: true,
        message: "Parser wird kalibriert".into(),
        fields: CalibratingMetadata::new(save_name),
    }
}
