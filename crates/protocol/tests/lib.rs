//! Contract tests for the shared transport.

use nadir_protocol::{EngineRequest, Operation, TRANSPORT_VERSION};
use serde_json::json;

#[test]
fn request_round_trips_in_camel_case() {
    let request = EngineRequest {
        transport_version: TRANSPORT_VERSION,
        operation: Operation::Ping,
        payload: json!({"message": "hello"}),
    };
    let encoded = serde_json::to_string(&request).expect("request serializes");
    assert!(encoded.contains("transportVersion"));
    let decoded = serde_json::from_str::<EngineRequest>(&encoded).expect("request parses");
    assert_eq!(decoded, request);
}
