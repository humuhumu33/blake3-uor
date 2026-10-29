//! `B3-17`: verified slices. A slice of chunk N is Bao's layout for one
//! chunk: the input length, the parent nodes on the chunk's path from the
//! root, the chunk. The model's slices (`sliceOf`, certified Rust) are held
//! byte for byte to the `bao` crate's (hash mode) and to slices built from
//! the `blake3` crate's hazmat API (every mode); its checker (`sliceCheck`,
//! certified Rust) accepts them, Bao's decoder accepts the model's, and
//! planted defects are refused by both.

use std::io::{Cursor, Read as _};

use blake3::hazmat::HasherExt as _;
use blake3_check::value::{adt, as_bytes, bytes, nat};
use blake3_check::{
    engine_calls, lex_slice_check, lex_slice_of, oracle, Corpus, Mode, SliceRejection,
};

const KEY: [u8; 32] = *b"whats the Elvish word for friend";
const CONTEXT: &[u8] = b"BLAKE3 2019-12-27 16:29:52 test vectors context";

fn modes() -> [Mode<'static>; 3] {
    [Mode::Hash, Mode::Keyed(&KEY), Mode::Derive(CONTEXT)]
}

fn chunks(len: usize) -> usize {
    len.div_ceil(1024).max(1)
}

fn chunk_size(len: usize, chunk: usize) -> usize {
    (len - chunk * 1024).min(1024)
}

fn hasher(mode: Mode<'_>) -> blake3::Hasher {
    match mode {
        Mode::Hash => blake3::Hasher::new(),
        Mode::Keyed(key) => blake3::Hasher::new_keyed(key.try_into().expect("32 bytes")),
        Mode::Derive(context) => {
            blake3::Hasher::new_derive_key(std::str::from_utf8(context).expect("UTF-8"))
        }
    }
}

/// The CV of the subtree `input[lo..lo + len]`, by the hazmat API.
fn subtree(mode: Mode<'_>, input: &[u8], lo: usize, len: usize) -> [u8; 32] {
    let mut h = hasher(mode);
    h.set_input_offset(lo as u64);
    h.update(&input[lo..lo + len]);
    h.finalize_non_root()
}

fn left_len(len: usize) -> usize {
    usize::try_from(blake3::hazmat::left_subtree_len(len as u64)).expect("fits")
}

/// The oracle's slice: the Bao layout, every CV from the hazmat API and the
/// tree's shape from its `left_subtree_len`.
fn hazmat_slice(mode: Mode<'_>, input: &[u8], chunk: usize) -> Vec<u8> {
    let mut out = (input.len() as u64).to_le_bytes().to_vec();
    let (mut lo, mut len) = (0, input.len());
    while len > 1024 {
        let left = left_len(len);
        out.extend(subtree(mode, input, lo, left));
        out.extend(subtree(mode, input, lo + left, len - left));
        if chunk * 1024 < lo + left {
            len = left;
        } else {
            lo += left;
            len -= left;
        }
    }
    out.extend(&input[chunk * 1024..chunk * 1024 + chunk_size(input.len(), chunk)]);
    out
}

fn bao_slice(input: &[u8], chunk: usize) -> Vec<u8> {
    let (encoded, _) = bao::encode::encode(input);
    let span = chunk_size(input.len(), chunk).max(1) as u64;
    let mut extractor =
        bao::encode::SliceExtractor::new(Cursor::new(&encoded[..]), (chunk * 1024) as u64, span);
    let mut out = Vec::new();
    extractor
        .read_to_end(&mut out)
        .expect("bao slices its own encoding");
    out
}

/// What Bao's decoder makes of a slice: the chunk, or a refusal.
fn bao_reads(slice: &[u8], input_len: usize, chunk: usize, root: &[u8]) -> Option<Vec<u8>> {
    let hash = blake3::Hash::from_bytes(root.try_into().expect("32 bytes"));
    let span = chunk_size(input_len, chunk).max(1) as u64;
    let mut decoder =
        bao::decode::SliceDecoder::new(Cursor::new(slice), &hash, (chunk * 1024) as u64, span);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).ok().map(|_| out)
}

/// Compressions to hash a whole input: every block, and one per parent.
fn whole_cost(len: usize) -> usize {
    (0..chunks(len))
        .map(|c| chunk_size(len, c).div_ceil(64).max(1))
        .sum::<usize>()
        + chunks(len)
        - 1
}

/// The model's slices equal both oracles' and its checker accepts them;
/// Bao's decoder accepts the model's.
#[test]
fn slices_equal_the_oracles_and_check_b3_17() {
    let mut corpus = Corpus::new(0xB3_17);
    let mut proved = 0;
    for len in [
        0usize, 1, 1023, 1024, 1025, 2048, 2049, 3072, 3073, 4097, 5121, 8192, 8193, 16385, 31745,
    ] {
        let input = corpus.bytes(len);
        for mode in modes() {
            let root = oracle(mode, &input, 32);
            for c in 0..chunks(len) {
                let slice = lex_slice_of(mode, &input, c).expect("a 32-byte key");
                assert_eq!(
                    slice,
                    hazmat_slice(mode, &input, c),
                    "{mode:?} len {len} chunk {c}: hazmat"
                );
                let chunk = lex_slice_check(mode, &slice, c, len, &root)
                    .unwrap_or_else(|e| panic!("{mode:?} len {len} chunk {c}: {e:?}"));
                assert_eq!(
                    chunk,
                    input[c * 1024..c * 1024 + chunk_size(len, c)],
                    "the checked chunk"
                );
                if matches!(mode, Mode::Hash) {
                    assert_eq!(slice, bao_slice(&input, c), "len {len} chunk {c}: bao");
                    assert_eq!(
                        bao_reads(&slice, len, c, &root),
                        Some(chunk),
                        "Bao accepts len {len} chunk {c}"
                    );
                }
                proved += 1;
            }
            assert_eq!(
                lex_slice_of(mode, &input, chunks(len)),
                Ok(Vec::new()),
                "no chunk past the end"
            );
        }
    }
    // A 1 MiB + 1 input: 1025 chunks, so up to 11 parents on a path.
    let big = corpus.bytes(1 << 20 | 1);
    let mut deepest = (0, 0);
    for mode in modes() {
        let root = oracle(mode, &big, 32);
        for c in [0usize, 1, 511, 512, 700, 1023, 1024] {
            let slice = lex_slice_of(mode, &big, c).expect("a 32-byte key");
            assert_eq!(slice, hazmat_slice(mode, &big, c), "{mode:?} big chunk {c}");
            if matches!(mode, Mode::Hash) {
                assert_eq!(slice, bao_slice(&big, c), "big chunk {c}: bao");
            }
            let chunk = lex_slice_check(mode, &slice, c, big.len(), &root).expect("checks");
            assert_eq!(chunk, big[c * 1024..c * 1024 + chunk_size(big.len(), c)]);
            deepest = deepest.max(((slice.len() - 8 - chunk.len()) / 64, slice.len()));
            proved += 1;
        }
    }
    let (depth, size) = deepest;
    eprintln!("B3-17: {proved} slices equal hazmat's (and bao's in hash mode), checked, and Bao accepts them");
    eprintln!(
        "B3-17: chunk 700 of {} chunks: a {size}-byte slice, {depth} parents; checking it runs {} compressions, hashing the input {}",
        chunks(big.len()),
        depth + 16,
        whole_cost(big.len())
    );
}

/// Planted defects in a slice of chunk 700 of a 1 MiB + 1 input, each
/// refused with its own error, and by Bao's decoder where Bao can say.
#[test]
fn planted_slice_defects_are_refused_b3_17() {
    let big = Corpus::new(0xB3_17_02).bytes(1 << 20 | 1);
    let root = oracle(Mode::Hash, &big, 32);
    let slice = lex_slice_of(Mode::Hash, &big, 700).expect("hash mode");
    let depth = (slice.len() - 8 - 1024) / 64;
    // Whether chunk 700's subtree is the right child at each level.
    let mut right = Vec::new();
    let (mut lo, mut len) = (0usize, big.len());
    while len > 1024 {
        let left = left_len(len);
        right.push(700 * 1024 >= lo + left);
        if 700 * 1024 >= lo + left {
            lo += left;
            len -= left;
        } else {
            len = left;
        }
    }
    assert_eq!(right.len(), depth);
    type Plant = (String, Vec<u8>, usize, usize, Vec<u8>, SliceRejection, bool);
    let mut plants: Vec<Plant> = Vec::new();
    for level in 0..depth {
        let node = 8 + 64 * level;
        let (sibling, on_path) = if right[level] {
            (node, node + 32)
        } else {
            (node + 32, node)
        };
        let mut s = slice.clone();
        s[sibling + level % 32] ^= 0x40;
        plants.push((
            format!("sibling CV, level {level}"),
            s,
            700,
            big.len(),
            root.clone(),
            SliceRejection::Parent(level as u64),
            true,
        ));
        let mut s = slice.clone();
        s[on_path + 31 - level % 32] ^= 0x01;
        plants.push((
            format!("on-path CV, level {level}"),
            s,
            700,
            big.len(),
            root.clone(),
            SliceRejection::Parent(level as u64),
            true,
        ));
    }
    let mut s = slice.clone();
    s[8 + 64 * depth + 100] ^= 0x08;
    plants.push((
        "a chunk byte".into(),
        s,
        700,
        big.len(),
        root.clone(),
        SliceRejection::Chunk,
        true,
    ));
    plants.push((
        "one byte short".into(),
        slice[..slice.len() - 1].to_vec(),
        700,
        big.len(),
        root.clone(),
        SliceRejection::Size,
        true,
    ));
    let mut s = slice.clone();
    s.push(0);
    plants.push((
        "one byte long".into(),
        s,
        700,
        big.len(),
        root.clone(),
        SliceRejection::Size,
        false,
    ));
    plants.push((
        "the next chunk's index".into(),
        slice.clone(),
        701,
        big.len(),
        root.clone(),
        SliceRejection::Chunk,
        false,
    ));
    let mut r = root.clone();
    r[31] ^= 0x80;
    plants.push((
        "a wrong root".into(),
        slice.clone(),
        700,
        big.len(),
        r,
        SliceRejection::Parent(0),
        true,
    ));
    let mut s = slice.clone();
    s[0] ^= 1;
    plants.push((
        "a wrong length field".into(),
        s,
        700,
        big.len(),
        root.clone(),
        SliceRejection::Header,
        true,
    ));
    plants.push((
        "a longer input claimed".into(),
        slice.clone(),
        700,
        big.len() + 1,
        root.clone(),
        SliceRejection::Header,
        false,
    ));
    plants.push((
        "a 31-byte root".into(),
        slice.clone(),
        700,
        big.len(),
        root[..31].to_vec(),
        SliceRejection::RootLength,
        false,
    ));
    plants.push((
        "a chunk past the end".into(),
        slice.clone(),
        1025,
        big.len(),
        root.clone(),
        SliceRejection::OutOfRange,
        false,
    ));
    for (name, s, c, len, r, want, bao) in &plants {
        assert_eq!(
            lex_slice_check(Mode::Hash, s, *c, *len, r),
            Err(want.clone()),
            "{name}"
        );
        if *bao {
            assert_eq!(bao_reads(s, *len, *c, r), None, "Bao refuses {name}");
        }
    }
    assert_eq!(
        lex_slice_check(Mode::Keyed(&KEY[..31]), &slice, 700, big.len(), &root),
        Err(SliceRejection::KeyLength(31)),
        "a 31-byte key"
    );
    assert_eq!(lex_slice_of(Mode::Keyed(&KEY[..31]), &big, 700), Err(31));
    eprintln!(
        "B3-17: {} planted defects and a 31-byte key refused, each with its own error",
        plants.len()
    );
}

/// The committed Prism image's `sliceOf` and `sliceCheck` agree with the
/// certified packages.
#[test]
fn the_image_slices_alike_b3_17() {
    let input = Corpus::new(0xB3_17_03).bytes(2049);
    let root = oracle(Mode::Hash, &input, 32);
    let slice = lex_slice_of(Mode::Hash, &input, 2).expect("hash mode");
    let mut wrong = slice.clone();
    wrong[8 + 3] ^= 1;
    let got = engine_calls(&[
        (
            "sliceOf",
            serde_json::json!([adt(0, vec![]), bytes(&input), nat(2)]),
        ),
        (
            "sliceCheck",
            serde_json::json!([
                adt(0, vec![]),
                bytes(&slice),
                nat(2),
                nat(2049),
                bytes(&root)
            ]),
        ),
        (
            "sliceCheck",
            serde_json::json!([
                adt(0, vec![]),
                bytes(&wrong),
                nat(2),
                nat(2049),
                bytes(&root)
            ]),
        ),
    ]);
    assert_eq!(as_bytes(&got[0]["value"]), slice);
    assert_eq!(as_bytes(&got[1]["value"]), input[2048..]);
    assert_eq!(got[2]["kind"], "error");
    assert_eq!(got[2]["value"]["constructor"], 4, "parent");
    assert_eq!(got[2]["value"]["fields"][0]["value"], "0", "level 0");
}
