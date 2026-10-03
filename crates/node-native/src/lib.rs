//! N-API adapter for the TypeScript SDK.

use napi_derive::napi;

/// Run one transport request and return one transport response as JSON text.
#[napi]
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn invoke(request: String) -> String {
    nadir_engine::invoke(&request)
}
