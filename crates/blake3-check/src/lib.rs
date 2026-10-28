//! Shared fixtures for the BLAKE3 differential.
//!
//! Every hashing operation under test is the LexLean export: the packages
//! `blake3_*` are rendered by LexLean from `lexlean/src/Blake3.lex.tex` and
//! contain no handwritten Rust. This crate only feeds them inputs and compares
//! their outputs with two oracles, the official vectors and the `blake3`
//! crate. A disagreement fails the case; the oracle's digest is never adopted.

use std::path::{Path, PathBuf};

/// The official key, from `test_vectors.json`.
pub const OFFICIAL_KEY: &[u8; 32] = b"whats the Elvish word for friend";
/// The official derive-key context, from `test_vectors.json`.
pub const OFFICIAL_CONTEXT: &str = "BLAKE3 2019-12-27 16:29:52 test vectors context";

/// Every length where the tree changes shape: block and chunk edges, each
/// power-of-two chunk count and the one after it, and two uneven trees.
pub const BOUNDARY_LENGTHS: &[usize] = &[
    0, 1, 63, 64, 65, 1023, 1024, 1025, 2048, 2049, 3072, 3073, 4096, 4097, 5120, 5121, 6144, 6145,
    7168, 7169, 8192, 8193, 16384, 31744, 102400,
];

/// Output lengths: empty, short, the default, one block, past one block, the
/// official extended length, and a multi-block length that is not a multiple
/// of 64.
pub const OUTPUT_LENGTHS: &[usize] = &[0, 1, 31, 32, 33, 64, 65, 131, 1031];

/// The repository root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/blake3-check is two below the root")
        .to_path_buf()
}

/// One official case: the input length and the 131-byte output of each mode.
pub struct OfficialCase {
    /// The input is `input_len` bytes, byte `i` being `i % 251`.
    pub input_len: usize,
    /// `hash` mode output.
    pub hash: Vec<u8>,
    /// `keyed_hash` mode output, under [`OFFICIAL_KEY`].
    pub keyed_hash: Vec<u8>,
    /// `derive_key` mode output, under [`OFFICIAL_CONTEXT`].
    pub derive_key: Vec<u8>,
}

/// The official vectors, parsed against a closed shape. The file is
/// untrusted data: an unexpected key, key string, context, or digest length
/// is a panic, never a skipped case.
pub fn official_cases(path: &Path) -> Vec<OfficialCase> {
    let text = std::fs::read_to_string(path).expect("the official vectors are committed");
    let raw: serde_json::Value = serde_json::from_str(&text).expect("the vectors are JSON");
    let object = raw.as_object().expect("the vectors are an object");
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys, ["_comment", "cases", "context_string", "key"]);
    assert_eq!(raw["key"].as_str(), std::str::from_utf8(OFFICIAL_KEY).ok());
    assert_eq!(raw["context_string"].as_str(), Some(OFFICIAL_CONTEXT));
    let digest = |case: &serde_json::Value, mode: &str| {
        let text = case[mode].as_str().expect("a digest is a string");
        let bytes = from_hex(text);
        assert_eq!(bytes.len(), 131, "every official output is 131 bytes");
        bytes
    };
    raw["cases"]
        .as_array()
        .expect("cases is an array")
        .iter()
        .map(|case| {
            let fields: Vec<&str> = case
                .as_object()
                .expect("a case is an object")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(fields, ["derive_key", "hash", "input_len", "keyed_hash"]);
            OfficialCase {
                input_len: usize::try_from(case["input_len"].as_u64().expect("a length"))
                    .expect("a length fits"),
                hash: digest(case, "hash"),
                keyed_hash: digest(case, "keyed_hash"),
                derive_key: digest(case, "derive_key"),
            }
        })
        .collect()
}

/// The official input pattern.
pub fn official_input(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| u8::try_from(i % 251).expect("below 251"))
        .collect()
}

/// A seeded xorshift64* stream: the corpus is the same on every run.
pub struct Corpus(u64);

impl Corpus {
    /// A stream from `seed` (zero is replaced, xorshift has no zero state).
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    /// The next 64 pseudo-random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// `len` pseudo-random bytes.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64().to_le_bytes()[7]).collect()
    }

    /// A value below `bound` (which must be positive).
    pub fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next_u64() % u64::try_from(bound).expect("fits"))
            .expect("below a usize")
    }
}

/// Lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Bytes from lowercase hex; anything else is a panic.
pub fn from_hex(text: &str) -> Vec<u8> {
    assert!(
        text.len().is_multiple_of(2)
            && text
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "not lowercase hex: {text}"
    );
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn width(len: usize) -> u64 {
    u64::try_from(len).expect("a length fits in u64")
}

/// `hash` mode through the exported package.
pub fn lex_hash(message: &[u8], out_len: usize) -> Vec<u8> {
    blake3_hash::run(blake3_hash::Bytes::lit(message), width(out_len))
        .expect("no overflow below 2^64")
        .octets()
        .to_vec()
}

/// The default 32-byte digest as lowercase hex, through the exported package.
pub fn lex_hash_hex(message: &[u8]) -> String {
    let hex =
        blake3_hash_hex::run(blake3_hash_hex::Bytes::lit(message)).expect("no overflow below 2^64");
    String::from_utf8(hex.octets().to_vec()).expect("hex is ASCII")
}

/// `keyed_hash` mode through the exported package: the digest, or the length
/// of a key that is not 32 bytes.
pub fn lex_keyed_hash(key: &[u8], message: &[u8], out_len: usize) -> Result<Vec<u8>, u64> {
    match blake3_keyed_hash::run(
        blake3_keyed_hash::Bytes::lit(key),
        blake3_keyed_hash::Bytes::lit(message),
        width(out_len),
    )
    .expect("no overflow below 2^64")
    {
        Ok(digest) => Ok(digest.octets().to_vec()),
        Err(blake3_keyed_hash::Adt0::C0(len)) => Err(len),
    }
}

/// `derive_key` mode through the exported package.
pub fn lex_derive_key(context: &[u8], material: &[u8], out_len: usize) -> Vec<u8> {
    blake3_derive_key::run(
        blake3_derive_key::Bytes::lit(context),
        blake3_derive_key::Bytes::lit(material),
        width(out_len),
    )
    .expect("no overflow below 2^64")
    .octets()
    .to_vec()
}

/// A hashing mode of the incremental hasher.
#[derive(Clone, Copy, Debug)]
pub enum Mode<'a> {
    /// Plain hashing.
    Hash,
    /// Keyed hashing under a key of any length (only 32 is accepted).
    Keyed(&'a [u8]),
    /// Key derivation under a context string.
    Derive(&'a [u8]),
}

/// `init`, one `update` per piece, then `finalize`, through the exported
/// package: the digest, or the length of a rejected key.
pub fn lex_pieces(mode: Mode<'_>, pieces: &[&[u8]], out_len: usize) -> Result<Vec<u8>, u64> {
    use blake3_hash_pieces::{Adt0, Adt1, Bytes, List};
    let mode = match mode {
        Mode::Hash => Adt0::C0,
        Mode::Keyed(key) => Adt0::C1(Bytes::lit(key)),
        Mode::Derive(context) => Adt0::C2(Bytes::lit(context)),
    };
    let list = pieces.iter().rev().fold(List::nil(), |tail, piece| {
        List::cons(Bytes::lit(piece), tail)
    });
    match blake3_hash_pieces::run(mode, list, width(out_len)).expect("no overflow below 2^64") {
        Ok(digest) => Ok(digest.octets().to_vec()),
        Err(Adt1::C0(len)) => Err(len),
    }
}

/// The oracle: the `blake3` crate, `out_len` bytes of extended output.
pub fn oracle(mode: Mode<'_>, message: &[u8], out_len: usize) -> Vec<u8> {
    let mut hasher = match mode {
        Mode::Hash => blake3::Hasher::new(),
        Mode::Keyed(key) => {
            blake3::Hasher::new_keyed(key.try_into().expect("the oracle takes 32-byte keys"))
        }
        Mode::Derive(context) => blake3::Hasher::new_derive_key(
            std::str::from_utf8(context).expect("the oracle takes a UTF-8 context"),
        ),
    };
    hasher.update(message);
    let mut out = vec![0; out_len];
    hasher.finalize_xof().fill(&mut out);
    out
}
