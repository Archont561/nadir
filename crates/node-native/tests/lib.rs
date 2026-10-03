//! Contract test for the N-API adapter.

#[test]
fn adapter_forwards_to_the_shared_engine() {
    let response = nadir_node_native::invoke(
        r#"{"transportVersion":1,"operation":"ping","payload":{"language":"typescript"}}"#
            .to_owned(),
    );
    assert!(response.contains("typescript"));
    assert!(response.contains("\"ok\":true"));
}
