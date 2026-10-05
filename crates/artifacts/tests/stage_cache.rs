//! Integration tests for the V0 stage cache store: staging directories, atomic
//! promotion, byte-revalidating lookups, and user outputs as copies.
//!
//! These are the ADR-012 cache invariants: an invocation key names requested
//! work, a verified manifest names the bytes actually produced, a hit
//! revalidates those bytes and launches nothing, and the cache directory is
//! never handed out as a user output path.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nadir_artifacts::{
    ArtifactHash, EntryInvalid, InvocationKey, InvocationSpec, Lookup, ManifestError, NamedDigest,
    RuntimeIdentity, StageCache, StageCacheError,
};

fn temp_root(label: &str) -> PathBuf {
    let unique = format!(
        "nadir-artifacts-store-{}-{}-{}",
        label,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after the epoch")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&dir).expect("test root created");
    dir
}

fn write_file(root: &Path, relative: &str, bytes: &[u8], unix_mode: u32) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("relative path has a parent"))
        .expect("parent dir created");
    std::fs::write(&path, bytes).expect("file written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(unix_mode))
            .expect("mode set");
    }
    #[cfg(not(unix))]
    let _ = unix_mode;
    path
}

fn spec(stage: &str, config_value: &str) -> InvocationSpec {
    InvocationSpec::new(
        stage,
        "colmap-feature-extractor/1",
        "colmap.database/1",
        [NamedDigest::new(
            "imageset",
            ArtifactHash::of_bytes(b"imageset:v1"),
        )],
        [("max_features", config_value)],
        RuntimeIdentity::new(
            "colmap",
            "pixi/linux-64/cpu",
            "3.13.0",
            ArtifactHash::of_bytes(b"colmap-executable"),
            [("threads", "8"), ("gpu", "disabled")],
        ),
    )
}

fn features_spec() -> InvocationSpec {
    spec("features", "8192")
}

/// Stage a two-file tree shaped like a COLMAP feature stage output.
fn stage_features_output(staging: &Path) {
    write_file(staging, "database.db", b"colmap sqlite bytes", 0o600);
    write_file(staging, "descriptors/sift.bin", b"sift descriptors", 0o644);
}

const EXPECTED: &[&str] = &["database.db", "descriptors/sift.bin"];

/// Recursively collect files under `dir`.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)
            .expect("readable tree")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn index_entry_files(store_root: &Path) -> Vec<PathBuf> {
    files_under(&store_root.join("index"))
}

fn object_dirs(store_root: &Path) -> Vec<PathBuf> {
    // The promoted layout is `objects/<digest[0..2]>/<digest[2..]>`.
    let objects = store_root.join("objects");
    let mut dirs = Vec::new();
    if !objects.is_dir() {
        return dirs;
    }
    for shard in std::fs::read_dir(&objects)
        .expect("readable objects")
        .flatten()
    {
        let shard_path = shard.path();
        if !shard_path.is_dir() {
            continue;
        }
        for tree in std::fs::read_dir(&shard_path)
            .expect("readable shard")
            .flatten()
        {
            let tree_path = tree.path();
            if tree_path.is_dir() {
                dirs.push(tree_path);
            }
        }
    }
    dirs.sort();
    dirs
}

fn promoted_with_output(
    store: &StageCache,
    invocation: &InvocationSpec,
    bytes: (&[u8], &[u8]),
) -> Result<nadir_artifacts::Promoted, StageCacheError> {
    let staging = store.staging_dir(&invocation.stage).expect("staging dir");
    write_file(&staging, "database.db", bytes.0, 0o600);
    write_file(&staging, "descriptors/sift.bin", bytes.1, 0o644);
    store.promote(invocation, &staging, EXPECTED)
}

#[test]
fn un_promoted_work_is_a_miss_not_a_hit() {
    let root = temp_root("miss");
    let store = StageCache::open(&root).expect("store opens");

    let key = InvocationKey::of_spec(&features_spec());
    assert_eq!(store.lookup(&key).expect("lookup runs"), Lookup::Miss);

    // A staged-but-unpromoted tree is still a miss: partial work never reads as
    // complete, because the entry is written only after promotion.
    let staging = store.staging_dir("features").expect("staging dir");
    stage_features_output(&staging);
    assert_eq!(store.lookup(&key).expect("lookup runs"), Lookup::Miss);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn staging_directories_are_unique() {
    let root = temp_root("staging-unique");
    let store = StageCache::open(&root).expect("store opens");

    let first = store.staging_dir("features").expect("first staging dir");
    let second = store.staging_dir("features").expect("second staging dir");

    assert_ne!(first, second);
    assert!(first.is_dir() && second.is_dir());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn promoted_entries_hit_and_revalidate_bytes() {
    let root = temp_root("hit");
    let store = StageCache::open(&root).expect("store opens");
    let invocation = features_spec();
    let key = InvocationKey::of_spec(&invocation);

    let staging = store.staging_dir("features").expect("staging dir");
    stage_features_output(&staging);
    let promoted = store
        .promote(&invocation, &staging, EXPECTED)
        .expect("promotion succeeds");

    assert_eq!(promoted.invocation_key, key);
    assert_eq!(
        promoted.manifest.digest(),
        promoted.tree_digest,
        "the recorded tree digest is the manifest's digest"
    );

    let Lookup::Hit(hit) = store.lookup(&key).expect("lookup runs") else {
        panic!("a promoted entry must hit");
    };
    assert_eq!(hit.tree_digest(), promoted.tree_digest);
    assert_eq!(hit.manifest(), &promoted.manifest);
    assert_eq!(hit.stage(), "features");
    assert_eq!(hit.contract_version(), "colmap.database/1");

    // A hit launches nothing and writes nothing: no staging appears, the
    // object bytes are untouched, and a second lookup hits identically.
    let staging_after = files_under(&root.join("staging"));
    assert!(
        staging_after.is_empty(),
        "hits create no staging: {staging_after:?}"
    );
    let again = match store.lookup(&key).expect("lookup runs") {
        Lookup::Hit(hit) => hit,
        other => panic!("second lookup must hit too, got {other:?}"),
    };
    assert_eq!(again.manifest(), hit.manifest());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn request_identity_and_produced_bytes_are_separate_identities() {
    let root = temp_root("separation");
    let store = StageCache::open(&root).expect("store opens");

    // Two different requests (different config → different invocation keys)…
    let first_spec = spec("features", "8192");
    let second_spec = spec("features", "16384");
    let first_key = InvocationKey::of_spec(&first_spec);
    let second_key = InvocationKey::of_spec(&second_spec);
    assert_ne!(first_key, second_key);

    // …that happen to produce byte-identical output trees.
    let first = promoted_with_output(
        &store,
        &first_spec,
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("first promotion");
    let second = promoted_with_output(
        &store,
        &second_spec,
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("second promotion");

    assert_ne!(first.invocation_key, second.invocation_key);
    assert_eq!(
        first.tree_digest, second.tree_digest,
        "identical produced bytes have one tree identity regardless of the request"
    );
    assert_ne!(
        first.invocation_key.to_string(),
        first.tree_digest.to_string(),
        "an invocation key is never a tree digest"
    );

    // Two entries, one shared object: content addressing, not per-key copies.
    assert_eq!(index_entry_files(&root).len(), 2);
    assert_eq!(object_dirs(&root).len(), 1);

    // Both keys hit, both revalidate the same bytes.
    for key in [&first_key, &second_key] {
        assert!(matches!(
            store.lookup(key).expect("lookup runs"),
            Lookup::Hit(_)
        ));
    }

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn promotion_validates_the_staged_structure() {
    let root = temp_root("promote-validation");
    let store = StageCache::open(&root).expect("store opens");
    let invocation = features_spec();
    let key = InvocationKey::of_spec(&invocation);

    // Missing: the staged tree lacks an expected file.
    let staging = store.staging_dir("features").expect("staging dir");
    write_file(&staging, "database.db", b"colmap sqlite bytes", 0o600);
    assert_eq!(
        store.promote(&invocation, &staging, EXPECTED),
        Err(StageCacheError::MissingExpected {
            path: PathBuf::from("descriptors/sift.bin")
        })
    );

    // Extra: the staged tree has an undeclared file.
    let staging = store.staging_dir("features").expect("staging dir");
    stage_features_output(&staging);
    write_file(&staging, "bonus.txt", b"undeclared", 0o644);
    assert_eq!(
        store.promote(&invocation, &staging, EXPECTED),
        Err(StageCacheError::UndeclaredFile {
            path: PathBuf::from("bonus.txt")
        })
    );

    // Hostile: a symlink in the staged tree.
    #[cfg(unix)]
    {
        let staging = store.staging_dir("features").expect("staging dir");
        write_file(&staging, "database.db", b"colmap sqlite bytes", 0o600);
        std::fs::create_dir_all(staging.join("descriptors")).expect("dir created");
        std::os::unix::fs::symlink("/etc/hostname", staging.join("descriptors/sift.bin"))
            .expect("hostile symlink");
        assert!(matches!(
            store.promote(&invocation, &staging, EXPECTED),
            Err(StageCacheError::Manifest(
                ManifestError::IrregularFile { .. }
            ))
        ));
    }

    // Unsafe expected paths are rejected before any staging is read.
    assert!(matches!(
        store.promote(&invocation, Path::new("/tmp"), &["/etc/passwd"]),
        Err(StageCacheError::UnsafeExpectedPath { .. })
    ));

    // None of the failed promotions wrote an entry: still a miss.
    assert_eq!(store.lookup(&key).expect("lookup runs"), Lookup::Miss);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn foreign_staging_directories_are_refused() {
    let root = temp_root("foreign-staging");
    let store = StageCache::open(&root).expect("store opens");

    let outside = root.join("elsewhere");
    std::fs::create_dir_all(&outside).expect("dir created");
    stage_features_output(&outside);

    assert!(matches!(
        store.promote(&features_spec(), &outside, EXPECTED),
        Err(StageCacheError::ForeignStaging { .. })
    ));
    assert!(
        outside.exists(),
        "a refused promotion must not move or consume the caller's directory"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn identical_repromotion_is_idempotent() {
    let root = temp_root("repromote");
    let store = StageCache::open(&root).expect("store opens");
    let invocation = features_spec();
    let key = InvocationKey::of_spec(&invocation);

    let first = promoted_with_output(
        &store,
        &invocation,
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("first promotion");
    let second = promoted_with_output(
        &store,
        &invocation,
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("re-promotion of identical bytes is idempotent");

    assert_eq!(first.tree_digest, second.tree_digest);
    assert_eq!(object_dirs(&root).len(), 1);
    assert_eq!(index_entry_files(&root).len(), 1);

    assert!(matches!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Hit(_)
    ));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn same_key_with_different_bytes_is_refused() {
    let root = temp_root("conflict");
    let store = StageCache::open(&root).expect("store opens");
    let invocation = features_spec();
    let key = InvocationKey::of_spec(&invocation);

    let first = promoted_with_output(
        &store,
        &invocation,
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("first promotion");

    // Same request, different bytes: a reproducibility violation, not a
    // silently rewritten cache entry.
    let conflict = promoted_with_output(
        &store,
        &invocation,
        (b"colmap sqlite byres", b"sift descriptors"),
    );
    assert!(matches!(
        conflict,
        Err(StageCacheError::ConflictingEntry { .. })
    ));

    // The original verified entry still wins.
    let Lookup::Hit(hit) = store.lookup(&key).expect("lookup runs") else {
        panic!("the original entry must remain a hit");
    };
    assert_eq!(hit.tree_digest(), first.tree_digest);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn mutated_object_bytes_are_invalid_never_hits() {
    let root = temp_root("mutated");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let database = object_dirs(&root)[0].join("database.db");
    std::fs::write(&database, b"colmap sqlite byres").expect("bytes flipped");

    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::Manifest(ManifestError::DigestMismatch {
            path: PathBuf::from("database.db"),
            expected: ArtifactHash::of_bytes(b"colmap sqlite bytes").to_string(),
            found: ArtifactHash::of_bytes(b"colmap sqlite byres").to_string(),
        }))
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_or_extra_object_files_are_invalid_never_hits() {
    let root = temp_root("missing-extra");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let objects = object_dirs(&root);
    let sift = objects[0].join("descriptors/sift.bin");
    std::fs::remove_file(&sift).expect("file removed");
    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::Manifest(ManifestError::MissingFile {
            path: PathBuf::from("descriptors/sift.bin")
        }))
    );

    // Restore the manifest's bytes, then plant an undeclared file beside them.
    write_file(
        &objects[0],
        "descriptors/sift.bin",
        b"sift descriptors",
        0o644,
    );
    write_file(&objects[0], "surprise.txt", b"undeclared", 0o644);
    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::Manifest(ManifestError::ExtraFile {
            path: PathBuf::from("surprise.txt")
        }))
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_vanished_object_tree_is_invalid_never_a_hit() {
    let root = temp_root("vanished");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let objects = object_dirs(&root);
    std::fs::remove_dir_all(&objects[0]).expect("object tree removed");

    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::TreeMissing)
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn schema_incompatible_entries_are_invalid_never_hits() {
    let root = temp_root("schema");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let entry_path = index_entry_files(&root)[0].clone();

    // Unparseable entry bytes.
    std::fs::write(&entry_path, b"{ not json").expect("entry corrupted");
    assert!(matches!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::Schema { .. })
    ));

    // A well-formed document from a different format generation.
    std::fs::write(
        &entry_path,
        br#"{ "format": "nadir.stage-entry.v0", "stage": "features" }"#,
    )
    .expect("foreign format written");
    assert!(matches!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::Schema { .. })
    ));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn tampered_entry_fields_are_invalid_never_hits() {
    let root = temp_root("tampered");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    let promoted = promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let entry_path = index_entry_files(&root)[0].clone();

    // A tree digest that does not match the manifest it claims to verify.
    let lying_digest = ArtifactHash::of_bytes(b"a-different-tree").to_string();
    let entry_bytes = std::fs::read_to_string(&entry_path).expect("entry readable");
    let mut entry: serde_json::Value = serde_json::from_str(&entry_bytes).expect("entry parses");
    entry["tree_digest"] = serde_json::Value::String(lying_digest.clone());
    std::fs::write(
        &entry_path,
        serde_json::to_string(&entry).expect("reserializes"),
    )
    .expect("entry rewritten");

    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::TreeDigestMismatch {
            recorded: lying_digest,
            computed: promoted.tree_digest.to_string(),
        })
    );

    // An entry file whose recorded key is not the key it is filed under.
    let other_key = InvocationKey::of_spec(&spec("features", "4096")).to_string();
    let mut entry: serde_json::Value = serde_json::from_str(&entry_bytes).expect("entry parses");
    entry["tree_digest"] = serde_json::Value::String(promoted.tree_digest.to_string());
    entry["invocation_key"] = serde_json::Value::String(other_key.clone());
    std::fs::write(
        &entry_path,
        serde_json::to_string(&entry).expect("reserializes"),
    )
    .expect("entry rewritten");

    assert_eq!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Invalid(EntryInvalid::KeyMismatch {
            requested: key.to_string(),
            recorded: other_key,
        })
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn materialize_copies_bytes_to_user_paths_without_exposing_the_store() {
    let root = temp_root("materialize");
    let store = StageCache::open(&root).expect("store opens");
    let key = InvocationKey::of_spec(&features_spec());
    promoted_with_output(
        &store,
        &features_spec(),
        (b"colmap sqlite bytes", b"sift descriptors"),
    )
    .expect("promotion");

    let Lookup::Hit(hit) = store.lookup(&key).expect("lookup runs") else {
        panic!("expected a hit");
    };

    let outputs = root.join("outputs/features");
    store.materialize(&hit, &outputs).expect("outputs copied");

    let copied_database = outputs.join("database.db");
    let copied_sift = outputs.join("descriptors/sift.bin");
    assert_eq!(
        std::fs::read(&copied_database).expect("copied file readable"),
        b"colmap sqlite bytes"
    );
    assert_eq!(
        std::fs::read(&copied_sift).expect("copied file readable"),
        b"sift descriptors"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&copied_database)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "recorded modes travel with the copy");
    }

    // The copy is user territory: mutating it cannot corrupt the store.
    std::fs::write(&copied_database, b"user edits are fine").expect("user edits their copy");
    assert!(matches!(
        store.lookup(&key).expect("lookup runs"),
        Lookup::Hit(_)
    ));

    std::fs::remove_dir_all(&root).ok();
}
