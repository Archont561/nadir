//! The labeled-hash encoding shared by nadir-artifacts' identity types.
//!
//! This mirrors the byte-level scheme of `nadir-core`'s `ArtifactHash::of_task`
//! — every field is written as `label`, NUL, the field length as little-endian
//! `u64`, then the bytes — so no field can bleed into its neighbor. The helpers
//! here buffer the labeled stream and hand it to [`ArtifactHash::of_bytes`],
//! which keeps the one blake3 implementation in the workspace inside
//! `nadir-core` and this crate free of a second one.

use nadir_core::ArtifactHash;

/// Append a labeled string field.
pub(crate) fn update_str(buffer: &mut Vec<u8>, label: &str, value: &str) {
    update_bytes(buffer, label, value.as_bytes());
}

/// Append a labeled byte field.
pub(crate) fn update_bytes(buffer: &mut Vec<u8>, label: &str, bytes: &[u8]) {
    buffer.extend_from_slice(label.as_bytes());
    buffer.push(0);
    buffer.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buffer.extend_from_slice(bytes);
}

/// Append a labeled length field (a count, not bytes).
pub(crate) fn update_len(buffer: &mut Vec<u8>, label: &str, len: usize) {
    buffer.extend_from_slice(label.as_bytes());
    buffer.push(0);
    buffer.extend_from_slice(&(len as u64).to_le_bytes());
}

/// Finalize a labeled stream into a digest.
pub(crate) fn finalize(buffer: &[u8]) -> ArtifactHash {
    ArtifactHash::of_bytes(buffer)
}
