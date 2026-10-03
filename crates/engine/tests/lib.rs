//! Contract tests for the shared dispatcher.

use serde_json::{Value, json};

fn invoke(operation: &str, payload: Value) -> Value {
    let request = json!({"transportVersion": 1, "operation": operation, "payload": payload});
    serde_json::from_str(&nadir_engine::invoke(&request.to_string())).expect("valid response")
}

#[test]
fn ping_echoes_arbitrary_json() {
    let response = invoke("ping", json!({"message": "hello", "count": 2}));
    assert_eq!(response["ok"], true);
    assert_eq!(response["result"]["echo"]["count"], 2);
}

#[test]
fn rejects_unknown_transport_versions() {
    let response: Value = serde_json::from_str(&nadir_engine::invoke(
        r#"{"transportVersion":99,"operation":"ping","payload":{}}"#,
    ))
    .expect("valid response");
    assert_eq!(response["ok"], false);
    assert_eq!(response["result"]["supported"], 1);
}
