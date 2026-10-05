//! Integration tests for the V0 invocation-key seam: request identity for a
//! stage execution, built from task/contract versions, named input digests,
//! normalized configuration, qualified runtime identity and every
//! byte-affecting setting.
//!
//! ADR-012 keeps this identity strictly separate from the digests that name
//! produced output bytes; the separation itself is tested at the store seam in
//! [`stage_cache.rs`](./stage_cache.rs).

use nadir_artifacts::{
    ArtifactHash, InvocationKey, InvocationSpec, NamedDigest, RuntimeIdentity, Setting,
};

fn digest_of(label: &str) -> ArtifactHash {
    ArtifactHash::of_bytes(label.as_bytes())
}

fn sample_runtime() -> RuntimeIdentity {
    RuntimeIdentity::new(
        "colmap",
        "pixi/linux-64/cpu",
        "3.13.0",
        digest_of("colmap-executable"),
        [("threads", "8"), ("gpu", "disabled"), ("locale", "C")],
    )
}

fn sample_spec() -> InvocationSpec {
    InvocationSpec::new(
        "features",
        "colmap-feature-extractor/1",
        "colmap.database/1",
        [
            NamedDigest::new("imageset", digest_of("imageset:v1")),
            NamedDigest::new("intrinsics", digest_of("intrinsics:v1")),
        ],
        [("detector", "sift"), ("max_features", "8192")],
        sample_runtime(),
    )
}

#[test]
fn invocation_key_is_deterministic_hex_identity() {
    let key = InvocationKey::of_spec(&sample_spec());

    assert_eq!(key, InvocationKey::of_spec(&sample_spec()));
    let hex = key.to_string();
    assert_eq!(hex.len(), 64, "invocation keys are blake3 hex digests");
    assert!(hex.bytes().all(|b| b.is_ascii_hexdigit()));

    let parsed: InvocationKey = hex.parse().expect("hex round-trips");
    assert_eq!(parsed, key);
    assert!("not-a-key".parse::<InvocationKey>().is_err());
}

#[test]
fn member_order_does_not_change_the_key() {
    let base = InvocationKey::of_spec(&sample_spec());

    let reordered = InvocationKey::of_spec(&InvocationSpec::new(
        "features",
        "colmap-feature-extractor/1",
        "colmap.database/1",
        [
            NamedDigest::new("intrinsics", digest_of("intrinsics:v1")),
            NamedDigest::new("imageset", digest_of("imageset:v1")),
        ],
        [("max_features", "8192"), ("detector", "sift")],
        RuntimeIdentity::new(
            "colmap",
            "pixi/linux-64/cpu",
            "3.13.0",
            digest_of("colmap-executable"),
            [("locale", "C"), ("gpu", "disabled"), ("threads", "8")],
        ),
    ));

    assert_eq!(base, reordered, "normalized members hash identically");
}

#[test]
fn every_byte_affecting_identity_input_changes_the_key() {
    let base = sample_spec();
    let base_key = InvocationKey::of_spec(&base);

    let runtime_with = |engine: &str, profile: &str, version: &str, digest: ArtifactHash| {
        RuntimeIdentity::new(
            engine,
            profile,
            version,
            digest,
            [("threads", "8"), ("gpu", "disabled"), ("locale", "C")],
        )
    };

    let variants: Vec<(&str, InvocationSpec)> = vec![
        {
            let mut spec = base.clone();
            spec.stage = "matching".to_owned();
            ("stage name", spec)
        },
        {
            let mut spec = base.clone();
            spec.task_version = "colmap-feature-extractor/2".to_owned();
            ("task version", spec)
        },
        {
            let mut spec = base.clone();
            spec.contract_version = "colmap.database/2".to_owned();
            ("contract version", spec)
        },
        {
            let mut spec = base.clone();
            spec.inputs[0].name = "frames".to_owned();
            ("input name", spec)
        },
        {
            let mut spec = base.clone();
            spec.inputs[0].digest = digest_of("imageset:v2");
            ("input digest", spec)
        },
        {
            let mut spec = base.clone();
            spec.config[1].value = "16384".to_owned();
            ("config value", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime.engine = "openga".to_owned();
            ("runtime engine", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime.profile = "oci@sha256:f00d".to_owned();
            ("runtime profile", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime.version = "3.13.1".to_owned();
            ("runtime version", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime.executable_digest = digest_of("a-different-executable");
            ("runtime executable digest", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime.settings[0].value = "16".to_owned();
            ("runtime byte-affecting setting", spec)
        },
        {
            let mut spec = base.clone();
            spec.runtime = runtime_with(
                "colmap",
                "pixi/linux-64/cpu",
                "3.13.0",
                digest_of("colmap-executable"),
            );
            spec.runtime.settings.push(Setting::new("timezone", "UTC"));
            ("an added runtime setting", spec)
        },
        {
            let mut spec = base.clone();
            spec.inputs
                .push(NamedDigest::new("extra", digest_of("extra")));
            ("an added input", spec)
        },
    ];

    assert!(!variants.is_empty());
    for (what_changed, spec) in variants {
        let key = InvocationKey::of_spec(&spec);
        assert_ne!(
            base_key, key,
            "changing {what_changed} must invalidate the invocation key"
        );
    }
}

#[test]
fn members_are_normalized_before_hashing() {
    let base = InvocationKey::of_spec(&sample_spec());

    let noisy = InvocationKey::of_spec(&InvocationSpec::new(
        " features ",
        " colmap-feature-extractor/1 ",
        " colmap.database/1 ",
        [
            NamedDigest::new(" imageset ", digest_of("imageset:v1")),
            NamedDigest::new("intrinsics", digest_of("intrinsics:v1")),
        ],
        [("  detector", "sift"), ("max_features ", " 8192 ")],
        RuntimeIdentity::new(
            " colmap ",
            " pixi/linux-64/cpu ",
            " 3.13.0 ",
            digest_of("colmap-executable"),
            [("threads", " 8"), ("gpu", " disabled"), ("locale", " C ")],
        ),
    ));

    assert_eq!(
        base, noisy,
        "whitespace noise is normalized away before hashing"
    );
}

#[test]
fn specs_serialize_to_stable_human_readable_json() {
    let spec = sample_spec();
    let json = serde_json::to_string_pretty(&spec).expect("spec serializes");

    for expected in [
        "\"stage\": \"features\"",
        "\"task_version\": \"colmap-feature-extractor/1\"",
        "\"contract_version\": \"colmap.database/1\"",
        "\"name\": \"imageset\"",
        "\"profile\": \"pixi/linux-64/cpu\"",
        "\"executable_digest\"",
    ] {
        assert!(
            json.contains(expected),
            "expected {expected} in serialized spec:\n{json}"
        );
    }

    let round_tripped: InvocationSpec = serde_json::from_str(&json).expect("spec deserializes");
    assert_eq!(round_tripped, spec);
    assert_eq!(
        InvocationKey::of_spec(&round_tripped),
        InvocationKey::of_spec(&spec),
        "a round-tripped spec keeps its invocation key"
    );
}

#[test]
fn keys_serialize_as_plain_hex_strings() {
    let key = InvocationKey::of_spec(&sample_spec());

    let json = serde_json::to_string(&key).expect("key serializes");
    assert_eq!(json, format!("\"{key}\""));

    let parsed: InvocationKey = serde_json::from_str(&json).expect("key deserializes");
    assert_eq!(parsed, key);
}
