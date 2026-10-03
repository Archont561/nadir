//! Contract test for the PyO3 adapter.

#[test]
fn adapter_forwards_to_the_shared_engine() {
    let response = nadir_python_native::invoke(
        r#"{"transportVersion":1,"operation":"ping","payload":{"language":"python"}}"#,
    );
    assert!(response.contains("python"));
    assert!(response.contains("\"ok\":true"));
}
