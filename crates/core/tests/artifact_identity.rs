//! Integration tests for the public artifact identity and serialization seam.

use std::path::PathBuf;

use nadir_core::artifact::{Artifact, ArtifactHash, ArtifactKind, TaskSpec};
use serde_json::json;

fn input_hash(label: &str) -> ArtifactHash {
    ArtifactHash::of_bytes(label.as_bytes())
}

#[test]
fn content_hash_uses_blake3_hex_encoding() {
    assert_eq!(
        ArtifactHash::of_bytes(b"").to_string(),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
    assert_eq!(
        ArtifactHash::of_bytes(b"abc").to_string(),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
    );
}

fn sample_spec(params: impl IntoIterator<Item = (&'static str, &'static str)>) -> TaskSpec {
    TaskSpec::new(
        "extract_features",
        [input_hash("images:v1")],
        params,
        "colmap 3.13.0",
    )
}

#[test]
fn equivalent_specs_hash_identically_when_parameter_order_differs() {
    let first = ArtifactHash::of_task(&sample_spec([
        ("max_features", "8192"),
        ("detector", "sift"),
    ]));
    let second = ArtifactHash::of_task(&sample_spec([
        ("detector", "sift"),
        ("max_features", "8192"),
    ]));

    assert_eq!(first, second);
}

#[test]
fn task_hash_changes_when_identity_inputs_change() {
    let base = ArtifactHash::of_task(&TaskSpec::new(
        "extract_features",
        [input_hash("images:v1"), input_hash("camera:v1")],
        [("detector", "sift")],
        "colmap 3.13.0",
    ));

    for changed in [
        TaskSpec::new(
            "match_features",
            [input_hash("images:v1"), input_hash("camera:v1")],
            [("detector", "sift")],
            "colmap 3.13.0",
        ),
        TaskSpec::new(
            "extract_features",
            [input_hash("camera:v1"), input_hash("images:v1")],
            [("detector", "sift")],
            "colmap 3.13.0",
        ),
        TaskSpec::new(
            "extract_features",
            [input_hash("images:v1"), input_hash("camera:v1")],
            [("detector", "orb")],
            "colmap 3.13.0",
        ),
        TaskSpec::new(
            "extract_features",
            [input_hash("images:v1"), input_hash("camera:v1")],
            [("detector", "sift")],
            "colmap 3.13.1",
        ),
    ] {
        assert_ne!(base, ArtifactHash::of_task(&changed));
    }
}

#[test]
fn artifact_metadata_serializes_to_stable_human_readable_json() {
    let spec = sample_spec([("detector", "sift"), ("max_features", "8192")]);
    let hash = ArtifactHash::of_task(&spec);
    let artifact = Artifact {
        hash,
        kind: ArtifactKind::Features,
        path: PathBuf::from("artifacts/features.json"),
        size_bytes: 42,
        produced_by: Some(spec),
    };

    let hash_json = serde_json::to_string(&hash).expect("hash serializes");
    assert_eq!(hash_json, format!("\"{hash}\""));
    assert_eq!(
        serde_json::from_str::<ArtifactHash>(&hash_json).expect("hash deserializes"),
        hash
    );

    let pretty =
        serde_json::to_string_pretty(&artifact).expect("artifact serializes as pretty JSON");
    assert_eq!(
        pretty,
        format!(
            "{{\n  \"hash\": \"{hash}\",\n  \"kind\": \"features\",\n  \"path\": \"artifacts/features.json\",\n  \"size_bytes\": 42,\n  \"produced_by\": {{\n    \"kind\": \"extract_features\",\n    \"inputs\": [\n      \"{}\"\n    ],\n    \"params\": [\n      {{\n        \"name\": \"detector\",\n        \"value\": \"sift\"\n      }},\n      {{\n        \"name\": \"max_features\",\n        \"value\": \"8192\"\n      }}\n    ],\n    \"engine_version\": \"colmap 3.13.0\"\n  }}\n}}",
            input_hash("images:v1")
        )
    );

    let value = serde_json::to_value(&artifact).expect("artifact serializes");

    assert_eq!(
        value,
        json!({
            "hash": hash.to_string(),
            "kind": "features",
            "path": "artifacts/features.json",
            "size_bytes": 42,
            "produced_by": {
                "kind": "extract_features",
                "inputs": [input_hash("images:v1").to_string()],
                "params": [
                    { "name": "detector", "value": "sift" },
                    { "name": "max_features", "value": "8192" }
                ],
                "engine_version": "colmap 3.13.0"
            }
        })
    );
    assert_eq!(hash.to_string().len(), 64);
}
