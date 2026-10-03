//! PyO3 adapter for the Python SDK.
//!
//! The deliberately tiny surface keeps all operation dispatch in `nadir-engine`.

use pyo3::prelude::*;

/// Run one transport request and return one transport response as JSON text.
#[pyfunction]
#[must_use]
pub fn invoke(request: &str) -> String {
    nadir_engine::invoke(request)
}

/// Extension module imported as `nadir._native`.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(invoke, module)?)?;
    Ok(())
}
