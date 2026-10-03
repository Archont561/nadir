//! The protocol dispatcher behind every native language binding.
//!
//! This crate translates transport requests into core calls. Domain rules stay in
//! `nadir-core`; Python and TypeScript only construct requests and read responses.

use nadir_protocol::{EngineRequest, EngineResponse, Operation, TRANSPORT_VERSION};
use serde_json::{Value, json};

/// Run one request and return one response, both as JSON text.
#[must_use]
pub fn invoke(request_json: &str) -> String {
    let response = match serde_json::from_str::<EngineRequest>(request_json) {
        Ok(request) if request.transport_version == TRANSPORT_VERSION => dispatch(request),
        Ok(request) => failure(json!({
            "error": "unsupported transport version",
            "supported": TRANSPORT_VERSION,
            "received": request.transport_version,
        })),
        Err(error) => failure(json!({
            "error": "invalid request",
            "detail": error.to_string(),
        })),
    };

    serde_json::to_string(&response).expect("engine responses contain only serializable JSON")
}

fn dispatch(request: EngineRequest) -> EngineResponse {
    match request.operation {
        Operation::Ping => success(json!({
            "engine": "nadir-engine",
            "echo": request.payload,
        })),
        Operation::Version => success(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "transportVersion": TRANSPORT_VERSION,
            "coreCrate": nadir_core::crate_name(),
        })),
        Operation::Describe => success(json!({
            "description": nadir_core::describe(),
            "stage": nadir_core::STAGE,
        })),
    }
}

fn success(result: Value) -> EngineResponse {
    EngineResponse {
        transport_version: TRANSPORT_VERSION,
        ok: true,
        result,
    }
}

fn failure(result: Value) -> EngineResponse {
    EngineResponse {
        transport_version: TRANSPORT_VERSION,
        ok: false,
        result,
    }
}
