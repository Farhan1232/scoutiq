//! Look for repeated record layouts without assuming game meaning.

use super::types::RecordPatternHint;

pub fn discover_record_patterns(data: &[u8], block_index: usize) -> Vec<RecordPatternHint> {
    let mut hints = Vec::new();
    for stride in [8u32, 16, 24, 32, 64] {
        if data.len() < (stride as usize) * 6 {
            continue;
        }
        let mut score = 0u32;
        let s = stride as usize;
        // Compare adjacent record prefixes for similarity.
        for i in 0..5 {
            let a = &data[i * s..i * s + 4.min(s)];
            let b = &data[(i + 1) * s..(i + 1) * s + 4.min(s)];
            if a == b {
                score += 1;
            }
        }
        if score >= 3 {
            hints.push(RecordPatternHint {
                block_index,
                offset: 0,
                stride: Some(stride),
                evidence: vec![format!(
                    "adjacent {stride}-byte records share prefix ({score}/5)"
                )],
            });
        }
    }
    hints
}

/// Stub until deterministic record schemas exist.
pub fn parser_ready() -> bool {
    false
}

pub fn status_message() -> String {
    "Record parser not ready — structure discovery only".into()
}
