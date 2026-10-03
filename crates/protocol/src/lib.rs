//! The versioned transport shared by every nadir binding.
//!
//! Bindings expose one JSON-in/JSON-out call instead of mirroring the Rust API. Adding an
//! engine operation therefore changes this enum and the dispatcher, not every FFI ABI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The shape version of the request and response envelopes.
pub const TRANSPORT_VERSION: u32 = 1;

/// One request sent through an FFI adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineRequest {
    /// Envelope version spoken by the caller.
    pub transport_version: u32,
    /// Operation to dispatch.
    pub operation: Operation,
    /// Operation arguments.
    #[serde(default)]
    pub payload: Value,
}

/// Operations currently available through every binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    /// Echo the payload, proving the complete boundary is live.
    Ping,
    /// Report engine, package, and transport versions.
    Version,
    /// Return the core's current description and stage.
    Describe,
}

/// One response returned through an FFI adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineResponse {
    /// Envelope version spoken by this engine.
    pub transport_version: u32,
    /// Whether dispatch succeeded.
    pub ok: bool,
    /// Operation result, or a structured error when `ok` is false.
    pub result: Value,
}
