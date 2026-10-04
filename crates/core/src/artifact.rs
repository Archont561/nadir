//! The artifact model: what a task produces, how it is identified, and where it lives.
//!
//! Artifacts are the unit of caching, invalidation and provenance. A task's output hash is
//! derived only from the task identity: task kind, ordered input artifact hashes,
//! normalized parameters and the engine version. Paths and wall-clock time stay out of the
//! hash so the same work has the same identity across machines and workspaces.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The content address of an artifact or task output.
///
/// The JSON form is a 64-character hex string. The underlying Blake3 value stays an
/// implementation detail so metadata manifests remain stable and greppable instead of
/// exposing Blake3's byte-array representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactHash(#[serde(with = "hash_hex")] blake3::Hash);

impl ArtifactHash {
    /// Hash the identity of a task: what it is, what it reads, how it is configured.
    ///
    /// The input hashes are fed to the hasher in their declared order. Parameters are
    /// normalized by [`TaskSpec::new`] before hashing, so TOML table order and caller
    /// iteration order cannot change the cache key.
    pub fn of_task(spec: &TaskSpec) -> Self {
        let mut hasher = blake3::Hasher::new();
        update_str(&mut hasher, "domain", "nadir.task.v1");
        update_str(&mut hasher, "kind", &spec.kind);

        update_len(&mut hasher, "inputs", spec.inputs.len());
        for input in &spec.inputs {
            update_bytes(&mut hasher, "input", input.0.as_bytes());
        }

        update_len(&mut hasher, "params", spec.params.len());
        for param in &spec.params {
            update_str(&mut hasher, "param.name", &param.name);
            update_str(&mut hasher, "param.value", &param.value);
        }

        update_str(&mut hasher, "engine_version", &spec.engine_version);
        Self(hasher.finalize())
    }

    /// Hash bytes that already are the artifact's content, such as an input manifest.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes))
    }

    /// Return the hex form used on disk and in human-readable JSON.
    pub fn to_hex(self) -> String {
        self.0.to_hex()
    }
}

impl std::fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl std::str::FromStr for ArtifactHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        blake3::Hash::from_hex(s)
            .map(Self)
            .map_err(|err| format!("not a blake3 artifact hash: {err}"))
    }
}

fn update_str(hasher: &mut blake3::Hasher, label: &str, value: &str) {
    update_bytes(hasher, label, value.as_bytes());
}

fn update_bytes(hasher: &mut blake3::Hasher, label: &str, bytes: &[u8]) {
    hasher.update(label.as_bytes());
    hasher.update(b"\0");
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn update_len(hasher: &mut blake3::Hasher, label: &str, len: usize) {
    hasher.update(label.as_bytes());
    hasher.update(b"\0");
    hasher.update(&(len as u64).to_le_bytes());
}

/// Serialize and deserialize Blake3 hashes as hex strings instead of byte arrays.
mod hash_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        hash: &super::blake3::Hash,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hash.to_hex())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<super::blake3::Hash, D::Error> {
        let hex = String::deserialize(deserializer)?;
        super::blake3::Hash::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

/// What kind of thing a task produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A validated, EXIF-stamped set of input images.
    Dataset,
    /// Keypoints and descriptors for one image.
    Features,
    /// Correspondence graph between images.
    Matches,
    /// A sparse reconstruction: cameras, images, points.
    SparseModel,
    /// A dense point cloud.
    DenseCloud,
    /// A triangular mesh.
    Mesh,
    /// A georeferenced raster surface.
    Raster,
    /// A georeferenced orthomosaic.
    Orthomosaic,
    /// A vector map product.
    Map,
    /// Any other engine output, named by the pipeline.
    Other,
}

/// Where an artifact is stored and how it was produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// The content address and cache key.
    pub hash: ArtifactHash,
    /// What this artifact represents.
    pub kind: ArtifactKind,
    /// Where the bytes live, relative to the workspace root.
    pub path: PathBuf,
    /// Size on disk, recorded so reports can total a run without stat-ing the tree.
    pub size_bytes: u64,
    /// The task that produced the artifact; `None` for supplied input artifacts.
    pub produced_by: Option<TaskSpecOwned>,
}

/// A normalized task parameter that participates in task hashing and provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskParameter {
    /// Parameter name after trimming surrounding whitespace.
    pub name: String,
    /// Parameter value after trimming surrounding whitespace.
    pub value: String,
}

impl TaskParameter {
    /// Normalize one parameter pair.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: normalize_param_part(name),
            value: normalize_param_part(value),
        }
    }
}

/// The identity of a task: what it is, what it reads and how it is configured.
///
/// `TaskSpec` is owned so it can be stored directly in artifact provenance. Its
/// constructor normalizes the parameters into deterministic key/value order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSpec {
    /// The stage or operation name. Renaming a task kind invalidates its artifacts.
    pub kind: String,
    /// Artifact hashes read by this task, in declared edge order.
    pub inputs: Vec<ArtifactHash>,
    /// Normalized parameters, sorted by name and then value.
    pub params: Vec<TaskParameter>,
    /// Version string for the engine implementation that produces the artifact.
    pub engine_version: String,
}

impl TaskSpec {
    /// Build a task spec with deterministic parameter normalization.
    pub fn new<K, I, P, N, V, E>(kind: K, inputs: I, params: P, engine_version: E) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = ArtifactHash>,
        P: IntoIterator<Item = (N, V)>,
        N: Into<String>,
        V: Into<String>,
        E: Into<String>,
    {
        let mut params: Vec<_> = params
            .into_iter()
            .map(|(name, value)| TaskParameter::new(name, value))
            .collect();
        params.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.value.cmp(&right.value))
        });

        Self {
            kind: normalize_param_part(kind),
            inputs: inputs.into_iter().collect(),
            params,
            engine_version: normalize_param_part(engine_version),
        }
    }
}

/// Owned task provenance recorded in artifact metadata.
pub type TaskSpecOwned = TaskSpec;

fn normalize_param_part(value: impl Into<String>) -> String {
    value.into().trim().to_owned()
}

/// A content-addressed store of artifacts.
///
/// The store is deliberately a directory of JSON manifests so a workspace stays readable
/// with ordinary filesystem tools and interrupted writes never corrupt a database.
#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// Open or create a store rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Return the path an artifact manifest with this hash occupies.
    pub fn path_for(&self, hash: ArtifactHash) -> PathBuf {
        let hex = hash.to_hex();
        self.root.join(&hex[..2]).join(&hex[2..])
    }

    /// Record an artifact's metadata manifest.
    pub fn record(&self, artifact: &Artifact) -> std::io::Result<()> {
        let path = self.path_for(artifact.hash);
        std::fs::create_dir_all(path.parent().expect("artifact path has a parent"))?;
        let manifest = serde_json::to_vec_pretty(artifact)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, manifest)
    }

    /// Look an artifact up by hash.
    pub fn get(&self, hash: ArtifactHash) -> Option<Artifact> {
        let bytes = std::fs::read(self.path_for(hash)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Return whether an artifact manifest is present without parsing it.
    pub fn contains(&self, hash: ArtifactHash) -> bool {
        self.path_for(hash).is_file()
    }

    /// Iterate over every readable artifact manifest in the store.
    pub fn iter(&self) -> impl Iterator<Item = Artifact> {
        let root = self.root.clone();
        ArtifactStoreIter { stack: vec![root] }
    }
}

struct ArtifactStoreIter {
    stack: Vec<PathBuf>,
}

impl Iterator for ArtifactStoreIter {
    type Item = Artifact;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(path) = self.stack.pop() {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        self.stack.push(entry.path());
                    }
                }
                continue;
            }

            let bytes = std::fs::read(path).ok()?;
            if let Ok(artifact) = serde_json::from_slice(&bytes) {
                return Some(artifact);
            }
        }
        None
    }
}

mod blake3 {
    const BLOCK_LEN: usize = 64;
    const CHUNK_LEN: usize = 1024;
    const OUT_LEN: usize = 32;

    const CHUNK_START: u32 = 1 << 0;
    const CHUNK_END: u32 = 1 << 1;
    const PARENT: u32 = 1 << 2;
    const ROOT: u32 = 1 << 3;

    const IV: [u32; 8] = [
        0x6A09_E667,
        0xBB67_AE85,
        0x3C6E_F372,
        0xA54F_F53A,
        0x510E_527F,
        0x9B05_688C,
        0x1F83_D9AB,
        0x5BE0_CD19,
    ];

    const MSG_SCHEDULE: [[usize; 16]; 7] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
        [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
        [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
        [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
        [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
        [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
    ];

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Hash([u8; OUT_LEN]);

    impl Hash {
        pub fn as_bytes(&self) -> &[u8; OUT_LEN] {
            &self.0
        }

        pub fn to_hex(self) -> String {
            let mut hex = String::with_capacity(OUT_LEN * 2);
            for byte in self.0 {
                push_hex_byte(&mut hex, byte);
            }
            hex
        }

        pub fn from_hex(hex: &str) -> Result<Self, FromHexError> {
            if hex.len() != OUT_LEN * 2 {
                return Err(FromHexError::InvalidLength {
                    expected: OUT_LEN * 2,
                    actual: hex.len(),
                });
            }

            let mut bytes = [0_u8; OUT_LEN];
            let hex_bytes = hex.as_bytes();
            for (index, byte) in bytes.iter_mut().enumerate() {
                let high = decode_hex_nibble(hex_bytes[index * 2])?;
                let low = decode_hex_nibble(hex_bytes[index * 2 + 1])?;
                *byte = (high << 4) | low;
            }
            Ok(Self(bytes))
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FromHexError {
        InvalidLength { expected: usize, actual: usize },
        InvalidCharacter { byte: u8 },
    }

    impl std::fmt::Display for FromHexError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                FromHexError::InvalidLength { expected, actual } => {
                    write!(
                        formatter,
                        "expected {expected} hex characters, got {actual}"
                    )
                }
                FromHexError::InvalidCharacter { byte } => {
                    write!(formatter, "invalid hex character 0x{byte:02x}")
                }
            }
        }
    }

    pub struct Hasher {
        bytes: Vec<u8>,
    }

    impl Hasher {
        pub fn new() -> Self {
            Self { bytes: Vec::new() }
        }

        pub fn update(&mut self, bytes: &[u8]) {
            self.bytes.extend_from_slice(bytes);
        }

        pub fn finalize(self) -> Hash {
            hash(&self.bytes)
        }
    }

    pub fn hash(bytes: &[u8]) -> Hash {
        if bytes.len() <= CHUNK_LEN {
            return chunk_output(bytes, 0).root_hash();
        }

        let mut cv_stack = Vec::new();
        let chunk_count = bytes.len().div_ceil(CHUNK_LEN);

        for (chunk_index, chunk) in bytes.chunks(CHUNK_LEN).take(chunk_count - 1).enumerate() {
            let chunk_cv = chunk_output(chunk, chunk_index as u64).chaining_value();
            add_chunk_chaining_value(&mut cv_stack, chunk_cv, chunk_index + 1);
        }

        let final_chunk_index = chunk_count - 1;
        let final_chunk = &bytes[final_chunk_index * CHUNK_LEN..];
        let mut output = chunk_output(final_chunk, final_chunk_index as u64);

        while let Some(left_cv) = cv_stack.pop() {
            output = parent_output(&left_cv, &output.chaining_value());
        }

        output.root_hash()
    }

    fn add_chunk_chaining_value(
        cv_stack: &mut Vec<[u32; 8]>,
        mut new_cv: [u32; 8],
        mut total_chunks: usize,
    ) {
        while total_chunks & 1 == 0 {
            let left_cv = cv_stack.pop().expect("left subtree is on the stack");
            new_cv = parent_output(&left_cv, &new_cv).chaining_value();
            total_chunks >>= 1;
        }
        cv_stack.push(new_cv);
    }

    #[derive(Clone, Copy)]
    struct Output {
        input_chaining_value: [u32; 8],
        block_words: [u32; 16],
        counter: u64,
        block_len: u32,
        flags: u32,
    }

    impl Output {
        fn chaining_value(self) -> [u32; 8] {
            let words = compress(
                &self.input_chaining_value,
                &self.block_words,
                self.counter,
                self.block_len,
                self.flags,
            );
            words[..8].try_into().expect("eight chaining words")
        }

        fn root_hash(self) -> Hash {
            let words = compress(
                &self.input_chaining_value,
                &self.block_words,
                self.counter,
                self.block_len,
                self.flags | ROOT,
            );
            let mut bytes = [0_u8; OUT_LEN];
            let (chunks, remainder) = bytes.as_chunks_mut::<4>();
            debug_assert!(remainder.is_empty());
            for (word, chunk) in words[..8].iter().zip(chunks.iter_mut()) {
                *chunk = word.to_le_bytes();
            }
            Hash(bytes)
        }
    }

    fn chunk_output(mut input: &[u8], chunk_counter: u64) -> Output {
        let mut cv = IV;
        let mut blocks_compressed = 0_u32;

        while input.len() > BLOCK_LEN {
            let block_words = bytes_to_words(&input[..BLOCK_LEN]);
            let mut flags = 0;
            if blocks_compressed == 0 {
                flags |= CHUNK_START;
            }
            cv = compress(&cv, &block_words, chunk_counter, BLOCK_LEN as u32, flags)[..8]
                .try_into()
                .expect("eight chaining words");
            blocks_compressed += 1;
            input = &input[BLOCK_LEN..];
        }

        let mut block = [0_u8; BLOCK_LEN];
        block[..input.len()].copy_from_slice(input);
        let block_words = bytes_to_words(&block);
        let mut flags = CHUNK_END;
        if blocks_compressed == 0 {
            flags |= CHUNK_START;
        }

        Output {
            input_chaining_value: cv,
            block_words,
            counter: chunk_counter,
            block_len: input.len() as u32,
            flags,
        }
    }

    fn parent_output(left_cv: &[u32; 8], right_cv: &[u32; 8]) -> Output {
        let mut block_words = [0_u32; 16];
        block_words[..8].copy_from_slice(left_cv);
        block_words[8..].copy_from_slice(right_cv);
        Output {
            input_chaining_value: IV,
            block_words,
            counter: 0,
            block_len: BLOCK_LEN as u32,
            flags: PARENT,
        }
    }

    fn compress(
        chaining_value: &[u32; 8],
        block_words: &[u32; 16],
        counter: u64,
        block_len: u32,
        flags: u32,
    ) -> [u32; 16] {
        let mut state = [0_u32; 16];
        state[..8].copy_from_slice(chaining_value);
        state[8..12].copy_from_slice(&IV[..4]);
        state[12] = counter as u32;
        state[13] = (counter >> 32) as u32;
        state[14] = block_len;
        state[15] = flags;

        for schedule in MSG_SCHEDULE {
            round(&mut state, block_words, &schedule);
        }

        for index in 0..8 {
            state[index] ^= state[index + 8];
            state[index + 8] ^= chaining_value[index];
        }

        state
    }

    fn round(state: &mut [u32; 16], message: &[u32; 16], schedule: &[usize; 16]) {
        g(
            state,
            0,
            4,
            8,
            12,
            message[schedule[0]],
            message[schedule[1]],
        );
        g(
            state,
            1,
            5,
            9,
            13,
            message[schedule[2]],
            message[schedule[3]],
        );
        g(
            state,
            2,
            6,
            10,
            14,
            message[schedule[4]],
            message[schedule[5]],
        );
        g(
            state,
            3,
            7,
            11,
            15,
            message[schedule[6]],
            message[schedule[7]],
        );
        g(
            state,
            0,
            5,
            10,
            15,
            message[schedule[8]],
            message[schedule[9]],
        );
        g(
            state,
            1,
            6,
            11,
            12,
            message[schedule[10]],
            message[schedule[11]],
        );
        g(
            state,
            2,
            7,
            8,
            13,
            message[schedule[12]],
            message[schedule[13]],
        );
        g(
            state,
            3,
            4,
            9,
            14,
            message[schedule[14]],
            message[schedule[15]],
        );
    }

    fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
        state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
        state[d] = (state[d] ^ state[a]).rotate_right(16);
        state[c] = state[c].wrapping_add(state[d]);
        state[b] = (state[b] ^ state[c]).rotate_right(12);
        state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
        state[d] = (state[d] ^ state[a]).rotate_right(8);
        state[c] = state[c].wrapping_add(state[d]);
        state[b] = (state[b] ^ state[c]).rotate_right(7);
    }

    fn bytes_to_words(bytes: &[u8]) -> [u32; 16] {
        let mut words = [0_u32; 16];
        let (chunks, remainder) = bytes.as_chunks::<4>();
        debug_assert!(remainder.is_empty());
        for (word, bytes) in words.iter_mut().zip(chunks.iter()) {
            *word = u32::from_le_bytes(*bytes);
        }
        words
    }

    fn push_hex_byte(hex: &mut String, byte: u8) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        hex.push(char::from(HEX[(byte >> 4) as usize]));
        hex.push(char::from(HEX[(byte & 0x0f) as usize]));
    }

    fn decode_hex_nibble(byte: u8) -> Result<u8, FromHexError> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err(FromHexError::InvalidCharacter { byte }),
        }
    }
}
