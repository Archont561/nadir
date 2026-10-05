//! Integration tests for the output-tree manifest seam: the record of bytes a
//! stage actually produced, and the validation that decides whether a tree on
//! disk still is that record.
//!
//! The manifest digest (`TreeDigest`) is the produced-bytes identity. It is
//! deliberately a different type — and a different hash domain — from
//! [`nadir_artifacts::InvocationKey`], which names the request.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nadir_artifacts::{ArtifactHash, FileEntry, ManifestError, OutputTreeManifest, TreeDigest};

fn temp_root(label: &str) -> PathBuf {
    let unique = format!(
        "nadir-artifacts-{}-{}-{}",
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

/// A two-file staged tree shaped like a COLMAP stage output.
fn stage_tree(root: &Path) {
    write_file(root, "database.db", b"colmap sqlite bytes", 0o600);
    write_file(root, "sparse/cameras.bin", b"camera records", 0o644);
}

#[test]
fn manifests_record_paths_sizes_modes_and_digests() {
    let root = temp_root("record");
    stage_tree(&root);

    let manifest = OutputTreeManifest::build_from_dir(&root).expect("tree manifests");

    // Sorted by path, so the manifest is a canonical order, not walk order.
    let paths: Vec<String> = manifest
        .files
        .iter()
        .map(|file| file.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(paths, ["database.db", "sparse/cameras.bin"]);

    let database = &manifest.files[0];
    assert_eq!(database.size_bytes, b"colmap sqlite bytes".len() as u64);
    assert_eq!(
        database.digest,
        ArtifactHash::of_bytes(b"colmap sqlite bytes")
    );

    #[cfg(unix)]
    assert_eq!(database.mode, 0o600);
    #[cfg(unix)]
    assert_eq!(manifest.files[1].mode, 0o644);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn tree_digests_identify_produced_bytes_not_requests() {
    let first = temp_root("digest-a");
    let second = temp_root("digest-b");
    stage_tree(&first);
    stage_tree(&second);

    let first_manifest = OutputTreeManifest::build_from_dir(&first).expect("manifest a");
    let second_manifest = OutputTreeManifest::build_from_dir(&second).expect("manifest b");

    // Identical bytes and modes in different directories: one tree identity.
    assert_eq!(first_manifest.digest(), second_manifest.digest());
    assert_eq!(first_manifest, second_manifest);

    // One mutated byte: a different produced-bytes identity.
    write_file(&second, "database.db", b"colmap sqlite byres", 0o600);
    let mutated = OutputTreeManifest::build_from_dir(&second).expect("mutated manifest");
    assert_ne!(first_manifest.digest(), mutated.digest());

    // Same bytes, different permission bits: a different manifest record.
    write_file(&second, "database.db", b"colmap sqlite bytes", 0o640);
    let repermissioned = OutputTreeManifest::build_from_dir(&second).expect("repermissioned");
    assert_ne!(first_manifest.digest(), repermissioned.digest());

    // A tree digest is never an invocation key: different types and domains.
    let tree_digest = first_manifest.digest().digest();
    assert_ne!(tree_digest.to_string(), String::new());

    std::fs::remove_dir_all(&first).ok();
    std::fs::remove_dir_all(&second).ok();
}

#[test]
fn tree_digests_serialize_as_hex_and_round_trip() {
    let root = temp_root("digest-hex");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");
    let digest = manifest.digest();

    let hex = digest.to_string();
    assert_eq!(hex.len(), 64);
    assert_eq!(hex.parse::<TreeDigest>().expect("parses"), digest);
    assert_eq!(
        serde_json::to_string(&digest).expect("serializes"),
        format!("\"{digest}\"")
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn intact_trees_validate() {
    let root = temp_root("validate-ok");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    manifest
        .validate_tree(&root)
        .expect("an intact tree validates");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_files_are_typed_validation_errors() {
    let root = temp_root("validate-missing");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    std::fs::remove_file(root.join("database.db")).expect("file removed");

    assert_eq!(
        manifest.validate_tree(&root),
        Err(ManifestError::MissingFile {
            path: PathBuf::from("database.db")
        })
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn extra_files_are_typed_validation_errors() {
    let root = temp_root("validate-extra");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    write_file(&root, "surprise.txt", b"not in the manifest", 0o644);

    assert_eq!(
        manifest.validate_tree(&root),
        Err(ManifestError::ExtraFile {
            path: PathBuf::from("surprise.txt")
        })
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn mutated_bytes_are_typed_validation_errors() {
    let root = temp_root("validate-mutated");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    std::fs::write(root.join("sparse/cameras.bin"), b"hacked records").expect("bytes flipped");

    let error = manifest
        .validate_tree(&root)
        .expect_err("mutated bytes never validate");
    assert_eq!(
        error,
        ManifestError::DigestMismatch {
            path: PathBuf::from("sparse/cameras.bin"),
            expected: ArtifactHash::of_bytes(b"camera records").to_string(),
            found: ArtifactHash::of_bytes(b"hacked records").to_string(),
        }
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn size_lies_in_the_manifest_are_typed_validation_errors() {
    let root = temp_root("validate-size");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    let mut lying = manifest.clone();
    lying.files[0].size_bytes += 1;

    assert_eq!(
        lying.validate_tree(&root),
        Err(ManifestError::SizeMismatch {
            path: PathBuf::from("database.db"),
            expected: b"colmap sqlite bytes".len() as u64 + 1,
            found: b"colmap sqlite bytes".len() as u64,
        })
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn unsafe_manifest_paths_are_typed_validation_errors() {
    let root = temp_root("validate-unsafe");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    for unsafe_path in ["/etc/passwd", "../escape", ""] {
        let mut hostile = manifest.clone();
        hostile.files.push(FileEntry {
            path: PathBuf::from(unsafe_path),
            size_bytes: 1,
            mode: 0o644,
            digest: ArtifactHash::of_bytes(b"x"),
        });

        assert_eq!(
            hostile.validate_tree(&root),
            Err(ManifestError::UnsafePath {
                path: PathBuf::from(unsafe_path)
            }),
            "manifest path {unsafe_path:?} must be rejected before any filesystem access"
        );
    }

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn symlinks_in_trees_are_typed_validation_errors() {
    let root = temp_root("validate-symlink");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("database.db"), root.join("alias.db"))
        .expect("symlink created");

    // A symlink is irregular even when it is also undeclared: the walk rejects
    // it before any set comparison can classify it as merely "extra".
    assert_eq!(
        manifest.validate_tree(&root),
        Err(ManifestError::IrregularFile {
            path: PathBuf::from("alias.db")
        })
    );

    // A symlink that *replaces* a manifest file is also irregular, never a hit.
    #[cfg(unix)]
    {
        std::fs::remove_file(root.join("alias.db")).expect("first symlink removed");
        std::fs::remove_file(root.join("database.db")).expect("original removed");
        std::os::unix::fs::symlink("/etc/hostname", root.join("database.db"))
            .expect("hostile symlink planted");

        assert_eq!(
            manifest.validate_tree(&root),
            Err(ManifestError::IrregularFile {
                path: PathBuf::from("database.db")
            })
        );
    }

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn manifests_serialize_to_stable_human_readable_json() {
    let root = temp_root("manifest-json");
    stage_tree(&root);
    let manifest = OutputTreeManifest::build_from_dir(&root).expect("manifest");

    let json = serde_json::to_string_pretty(&manifest).expect("manifest serializes");
    for expected in [
        "\"path\": \"database.db\"",
        "\"size_bytes\": 19",
        "\"mode\": 384",
        "\"digest\": \"",
    ] {
        assert!(
            json.contains(expected),
            "expected {expected} in serialized manifest:\n{json}"
        );
    }

    let round_tripped: OutputTreeManifest =
        serde_json::from_str(&json).expect("manifest deserializes");
    assert_eq!(round_tripped, manifest);
    assert_eq!(round_tripped.digest(), manifest.digest());

    std::fs::remove_dir_all(&root).ok();
}
