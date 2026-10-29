//! An independent reader of the serialised trace (`docs/trace.md`).
//!
//! The model computes the trace's addresses with its own BLAKE3, so a bug
//! there would give self-consistent but wrong labels. This reader shares no
//! code with the model: it walks the file by the documented layout and
//! recomputes every DigestSet with the `blake3` and `sha2` crates.

use sha2::Digest as _;

/// The file magic, `B3TRACE1`.
pub const MAGIC: &[u8; 8] = b"B3TRACE1";
/// Bytes in one step record, before any bit records.
pub const STEP: usize = 18;
/// Steps of one compression that carry 32 bit records at level 2: every add
/// and addMsg (6 per G) and every xor (4 per G, plus the 16 feed-forward).
pub const BIT_STEPS: usize = 7 * 8 * 10 + 16;
/// A word section at level 1: load (offset, length, 64 bytes), provenance,
/// seven rounds of a 16-byte map and 112 steps, 16 feed-forward steps, and
/// the 16 output words.
pub const WORD_SECTION: usize = 8 + 1 + 64 + 16 + 7 * (16 + 112 * STEP) + 16 * STEP + 64;
/// A word section at level 2.
pub const BIT_SECTION: usize = WORD_SECTION + BIT_STEPS * 32;

/// One blob of a trace file and where its parts sit.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Offset of the entry's length field.
    pub at: usize,
    /// The blob's bytes.
    pub blob: Vec<u8>,
    /// The DigestSet the file records for it: BLAKE3 then SHA-256.
    pub digest_set: [u8; 64],
    /// Offset of the checkpoint's word section, if it has one.
    pub section: Option<usize>,
}

impl Entry {
    /// The blob kind, its first byte.
    pub fn kind(&self) -> u8 {
        self.blob[0]
    }

    /// The `blake3:` label.
    pub fn label(&self) -> String {
        format!("blake3:{}", crate::to_hex(&self.digest_set[..32]))
    }
}

/// A parsed trace file.
#[derive(Clone, Debug)]
pub struct TraceFile {
    /// 0 tree, 1 word, 2 bit.
    pub level: u8,
    /// Every blob in file order.
    pub entries: Vec<Entry>,
}

fn u32_at(data: &[u8], at: usize) -> usize {
    usize::try_from(u32::from_le_bytes(
        data[at..at + 4].try_into().expect("4 bytes"),
    ))
    .expect("fits")
}

/// Reads a trace file by the documented layout. Any departure is a panic.
pub fn parse(data: &[u8]) -> TraceFile {
    assert_eq!(&data[..8], MAGIC, "magic");
    let level = data[8];
    assert!(level <= 2, "level {level}");
    let count = usize::try_from(u64::from_le_bytes(data[9..17].try_into().expect("8 bytes")))
        .expect("fits");
    let mut at = 17;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let len = u32_at(data, at);
        let blob = data[at + 4..at + 4 + len].to_vec();
        let digest_set: [u8; 64] = data[at + 4 + len..at + 68 + len]
            .try_into()
            .expect("64 bytes");
        let mut next = at + 68 + len;
        let section = (blob[0] == 1 && level > 0).then(|| {
            let start = next;
            next += if level == 1 {
                WORD_SECTION
            } else {
                BIT_SECTION
            };
            start
        });
        entries.push(Entry {
            at,
            blob,
            digest_set,
            section,
        });
        at = next;
    }
    assert_eq!(at, data.len(), "no bytes after the last entry");
    TraceFile { level, entries }
}

/// ADR-015's DigestSet of `bytes`, by the crates: BLAKE3 then SHA-256.
pub fn digest_set(bytes: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(blake3::hash(bytes).as_bytes());
    out[32..].copy_from_slice(&sha2::Sha256::digest(bytes));
    out
}

/// The root blob: the last entry, kind 6.
pub fn root(file: &TraceFile) -> &Entry {
    let last = file.entries.last().expect("a trace has a root");
    assert_eq!(last.kind(), 6, "the last blob is the root");
    last
}

/// The digest the root records: its last `outLen` bytes.
pub fn root_digest(file: &TraceFile) -> Vec<u8> {
    let blob = &root(file).blob;
    let out_len = usize::try_from(u64::from_le_bytes(
        blob[11..19].try_into().expect("8 bytes"),
    ))
    .expect("fits");
    blob[blob.len() - out_len..].to_vec()
}

/// The input DigestSet the root records.
pub fn root_input_set(file: &TraceFile) -> [u8; 64] {
    root(file).blob[19..83].try_into().expect("64 bytes")
}

fn label_at(blob: &[u8], at: usize) -> [u8; 32] {
    blob[at..at + 32].try_into().expect("32 bytes")
}

fn u64_at(blob: &[u8], at: usize) -> usize {
    usize::try_from(u64::from_le_bytes(
        blob[at..at + 8].try_into().expect("8 bytes"),
    ))
    .expect("fits")
}

/// Every label a blob names, by its documented layout.
pub fn references(blob: &[u8]) -> Vec<[u8; 32]> {
    match blob[0] {
        2 => (0..usize::from(blob[9]))
            .map(|k| label_at(blob, 10 + 32 * k))
            .collect(),
        3 => {
            let mut named = vec![label_at(blob, 17), label_at(blob, 49)];
            if blob[81] == 1 {
                named.push(label_at(blob, 82));
            }
            named
        }
        4 => vec![label_at(blob, 9)],
        5 => {
            let mut named = Vec::new();
            let mut at = 9;
            for _ in 0..u64_at(blob, 1) {
                if blob[at] == 0 {
                    named.push(label_at(blob, at + 1));
                    at += 33;
                } else {
                    at += 50;
                }
            }
            assert_eq!(at, blob.len(), "the events fill the blob");
            named
        }
        6 => {
            let mut named = vec![label_at(blob, 83), label_at(blob, 115)];
            let mut at = 147;
            if blob[2] == 2 {
                named.push(label_at(blob, 147));
                at = 179;
            }
            for k in 0..u64_at(blob, at) {
                named.push(label_at(blob, at + 8 + 32 * k));
            }
            named
        }
        _ => Vec::new(),
    }
}
