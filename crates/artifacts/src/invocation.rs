//! Invocation keys: the identity of a *requested* stage execution.
//!
//! ADR-012 separates two identities that a naive cache collapses into one:
//!
//! * an [`InvocationKey`] names the work that was requested — stage, task and
//!   contract versions, named input digests, normalized configuration, the
//!   qualified runtime identity, and every byte-affecting setting;
//! * an output-tree digest (see [`crate::manifest`]) names the bytes that were
//!   actually produced.
//!
//! The key is derived only from the request, so a key can never be presented as
//! proof that specific bytes exist: the store records both and revalidates the
//! bytes before any cache hit. Feeding a different config or runtime to the same
//! stage changes the key; producing identical bytes under two different requests
//! yields two keys mapping to one tree.

use nadir_core::ArtifactHash;

use crate::hashing::{update_bytes, update_len, update_str};
/// One named input digest participating in an invocation key.
///
/// Inputs are named, not merely ordered: `imageset` and `intrinsics` are
/// distinct edges of the request even if their digests collided, and renaming an
/// input is a different request.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct NamedDigest {
    /// The input's logical name, trimmed by [`NamedDigest::new`].
    pub name: String,
    /// The content digest of the input artifact.
    pub digest: ArtifactHash,
}

impl NamedDigest {
    /// Build a named digest, trimming the name.
    pub fn new(name: impl Into<String>, digest: ArtifactHash) -> Self {
        Self {
            name: name.into().trim().to_owned(),
            digest,
        }
    }
}

/// One normalized byte-affecting setting.
///
/// Settings cover both the stage's resolved configuration and the runtime-level
/// facts that can change output bytes (threads, locale, argv). Names and values
/// are trimmed so caller-side whitespace cannot fork the cache.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Setting {
    /// Setting name after trimming.
    pub name: String,
    /// Setting value after trimming.
    pub value: String,
}

impl Setting {
    /// Build a setting, trimming name and value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into().trim().to_owned(),
            value: value.into().trim().to_owned(),
        }
    }
}

fn normalize_settings<N, V, I>(settings: I) -> Vec<Setting>
where
    N: Into<String>,
    V: Into<String>,
    I: IntoIterator<Item = (N, V)>,
{
    let mut settings: Vec<_> = settings
        .into_iter()
        .map(|(name, value)| Setting::new(name, value))
        .collect();
    settings.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.value.cmp(&right.value))
    });
    settings
}

/// The qualified identity of the runtime a stage executes under.
///
/// `engine`, `profile`, `version` and `executable_digest` together name the
/// qualified toolchain (locked Pixi profile, digest-pinned OCI image, or a
/// future qualified runtime); an ambient `PATH` executable has no identity here
/// and therefore no cache entries. `settings` records the byte-affecting
/// runtime facts such as thread count, locale, timezone, fixed argv and the
/// GPU-disabled state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeIdentity {
    /// Engine name, e.g. `colmap`.
    pub engine: String,
    /// Qualified runtime source, e.g. `pixi/linux-64/cpu` or `oci@sha256:...`.
    pub profile: String,
    /// Engine version string.
    pub version: String,
    /// Digest of the executable bytes actually run.
    pub executable_digest: ArtifactHash,
    /// Byte-affecting runtime settings, sorted by name then value.
    pub settings: Vec<Setting>,
}

impl RuntimeIdentity {
    /// Build a runtime identity, trimming strings and sorting settings.
    pub fn new<N, V, E, P, W, I>(
        engine: E,
        profile: P,
        version: W,
        executable_digest: ArtifactHash,
        settings: I,
    ) -> Self
    where
        E: Into<String>,
        P: Into<String>,
        W: Into<String>,
        N: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (N, V)>,
    {
        Self {
            engine: engine.into().trim().to_owned(),
            profile: profile.into().trim().to_owned(),
            version: version.into().trim().to_owned(),
            executable_digest,
            settings: normalize_settings(settings),
        }
    }
}

/// The full identity of one requested stage execution.
///
/// Every field is byte-affecting by construction: task version (the adapter
/// implementation that writes the outputs), contract version (the output-tree
/// structure the stage promises), named input digests, resolved configuration
/// and the qualified runtime identity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InvocationSpec {
    /// The pipeline stage name, e.g. `features`.
    pub stage: String,
    /// Version of the task implementation producing the outputs.
    pub task_version: String,
    /// Version of the output contract the stage promises.
    pub contract_version: String,
    /// Named input digests, sorted by name then digest.
    pub inputs: Vec<NamedDigest>,
    /// Normalized resolved configuration, sorted by name then value.
    pub config: Vec<Setting>,
    /// The qualified runtime identity the stage must execute under.
    pub runtime: RuntimeIdentity,
}

impl InvocationSpec {
    /// Build a spec with deterministic normalization: strings trimmed, inputs
    /// sorted by name then digest, config sorted by name then value.
    #[allow(clippy::too_many_arguments)]
    pub fn new<K, T, C, I, N, V>(
        stage: K,
        task_version: T,
        contract_version: C,
        inputs: I,
        config: impl IntoIterator<Item = (N, V)>,
        runtime: RuntimeIdentity,
    ) -> Self
    where
        K: Into<String>,
        T: Into<String>,
        C: Into<String>,
        I: IntoIterator<Item = NamedDigest>,
        N: Into<String>,
        V: Into<String>,
    {
        let mut inputs: Vec<_> = inputs.into_iter().collect();
        inputs.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.digest.cmp(&right.digest))
        });

        Self {
            stage: stage.into().trim().to_owned(),
            task_version: task_version.into().trim().to_owned(),
            contract_version: contract_version.into().trim().to_owned(),
            inputs,
            config: normalize_settings(config),
            runtime,
        }
    }
}

/// The request identity of a stage invocation: a name for the work, never a
/// claim about produced bytes.
///
/// This is a distinct type from the output-tree digest on purpose. Passing one
/// where the other belongs is a compile error, because conflating them is the
/// exact bug ADR-012's cache semantics exist to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvocationKey(ArtifactHash);

impl InvocationKey {
    /// Derive the invocation key from a normalized spec.
    ///
    /// The domain label and length-prefixed fields mirror `nadir-core`'s
    /// `ArtifactHash::of_task` scheme, so no field can bleed into its neighbor
    /// and no other hash domain can collide with this one.
    pub fn of_spec(spec: &InvocationSpec) -> Self {
        let mut buffer = Vec::new();
        update_str(&mut buffer, "domain", "nadir.invocation.v1");
        update_str(&mut buffer, "stage", &spec.stage);
        update_str(&mut buffer, "task_version", &spec.task_version);
        update_str(&mut buffer, "contract_version", &spec.contract_version);

        update_len(&mut buffer, "inputs", spec.inputs.len());
        for input in &spec.inputs {
            update_str(&mut buffer, "input.name", &input.name);
            update_bytes(
                &mut buffer,
                "input.digest",
                input.digest.to_hex().as_bytes(),
            );
        }

        update_len(&mut buffer, "config", spec.config.len());
        for setting in &spec.config {
            update_str(&mut buffer, "config.name", &setting.name);
            update_str(&mut buffer, "config.value", &setting.value);
        }

        update_str(&mut buffer, "runtime.engine", &spec.runtime.engine);
        update_str(&mut buffer, "runtime.profile", &spec.runtime.profile);
        update_str(&mut buffer, "runtime.version", &spec.runtime.version);
        update_bytes(
            &mut buffer,
            "runtime.executable_digest",
            spec.runtime.executable_digest.to_hex().as_bytes(),
        );

        update_len(&mut buffer, "runtime.settings", spec.runtime.settings.len());
        for setting in &spec.runtime.settings {
            update_str(&mut buffer, "runtime.setting.name", &setting.name);
            update_str(&mut buffer, "runtime.setting.value", &setting.value);
        }

        Self(crate::hashing::finalize(&buffer))
    }

    /// The underlying digest, for callers that report or store it.
    pub fn digest(self) -> ArtifactHash {
        self.0
    }
}

impl std::fmt::Display for InvocationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for InvocationKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<ArtifactHash>()
            .map(Self)
            .map_err(|err| format!("not an invocation key (expected blake3 hex): {err}"))
    }
}

impl serde::Serialize for InvocationKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for InvocationKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ArtifactHash::deserialize(deserializer).map(Self)
    }
}
