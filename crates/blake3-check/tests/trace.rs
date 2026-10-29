//! `B3-06` to `B3-11`: the trace (`docs/trace.md`) against independent
//! hashes, its checker against real and planted traces, its determinism, and
//! its shared addresses.

use blake3_check::trace::{self, TraceFile, BIT_SECTION, STEP, WORD_SECTION};
use blake3_check::{
    check_against, engine_check, lex_trace_address, lex_trace_file, oracle, Corpus, Mode,
    TraceRejection,
};

const KEY: [u8; 32] = *b"whats the Elvish word for friend";
const CONTEXT: &[u8] = b"BLAKE3 2019-12-27 16:29:52 test vectors context";

fn modes() -> [Mode<'static>; 3] {
    [Mode::Hash, Mode::Keyed(&KEY), Mode::Derive(CONTEXT)]
}

fn file(mode: Mode<'_>, input: &[u8], out_len: usize, level: u64) -> (Vec<u8>, TraceFile) {
    let bytes = lex_trace_file(mode, &[input], out_len, level).expect("a valid mode");
    let parsed = trace::parse(&bytes);
    (bytes, parsed)
}

/// The root records the digest the hash computes and the input's DigestSet,
/// and the trace's address is the root's DigestSet.
#[test]
fn the_trace_records_the_digest_and_its_address_b3_06() {
    let mut corpus = Corpus::new(0xB3_06);
    for len in [0usize, 1, 64, 65, 1024, 1025, 2048, 4097] {
        let input = corpus.bytes(len);
        for mode in modes() {
            let (_, parsed) = file(mode, &input, 131, 0);
            assert_eq!(
                trace::root_digest(&parsed),
                oracle(mode, &input, 131),
                "{mode:?} len {len}"
            );
            assert_eq!(
                trace::root_input_set(&parsed),
                trace::digest_set(&input),
                "{mode:?} len {len}"
            );
            let address = lex_trace_address(mode, &[&input], 131).expect("a valid mode");
            assert_eq!(
                &address[..],
                &trace::root(&parsed).digest_set[..],
                "{mode:?} len {len}"
            );
        }
    }
}

/// Every DigestSet in a trace, at every level, equals BLAKE3 and SHA-256 of
/// its blob as the `blake3` and `sha2` crates compute them, and every label a
/// blob names belongs to a blob before it.
#[test]
fn every_digest_set_equals_the_crates_b3_07() {
    let mut corpus = Corpus::new(0xB3_07);
    let mut checked = 0;
    for (len, levels) in [
        (0usize, 0..3),
        (65, 0..3),
        (1025, 0..3),
        (4097, 0..2),
        (16384, 0..1),
    ] {
        let input = corpus.bytes(len);
        for mode in modes() {
            for level in levels.clone() {
                let (_, parsed) = file(mode, &input, 131, level);
                let mut seen: Vec<[u8; 32]> = Vec::new();
                for entry in &parsed.entries {
                    assert_eq!(
                        entry.digest_set,
                        trace::digest_set(&entry.blob),
                        "{mode:?} len {len} level {level} at {}",
                        entry.at
                    );
                    for named in trace::references(&entry.blob) {
                        assert!(
                            seen.contains(&named),
                            "{} names an unknown label",
                            entry.label()
                        );
                    }
                    seen.push(entry.digest_set[..32].try_into().expect("32 bytes"));
                    checked += 1;
                }
            }
        }
    }
    eprintln!("B3-07: {checked} DigestSets equal the blake3 and sha2 crates");
}

/// `checkTrace`, run by the image's engine, accepts every real trace the
/// certified `traceFile` writes, at every level, and returns its address.
#[test]
fn check_trace_accepts_real_traces_b3_08() {
    let mut corpus = Corpus::new(0xB3_08);
    let mut files = Vec::new();
    let mut addresses = Vec::new();
    for (len, levels) in [(0usize, 0..3), (64, 0..3), (1025, 0..1)] {
        let input = corpus.bytes(len);
        for mode in modes() {
            let address = lex_trace_address(mode, &[&input], 100).expect("a valid mode");
            for level in levels.clone() {
                files.push(file(mode, &input, 100, level).0);
                addresses.push(address.clone());
            }
        }
    }
    let verdicts = engine_check(&files);
    for (k, (verdict, address)) in verdicts.iter().zip(&addresses).enumerate() {
        assert_eq!(verdict.as_ref(), Ok(address), "trace {k}");
    }
    let input = corpus.bytes(64);
    let whole = file(Mode::Hash, &input, 32, 0).0;
    assert!(check_against(Mode::Hash, &[&input], 32, &whole).is_ok());
    eprintln!(
        "B3-08: {} real traces accepted by checkTrace, each at its address",
        files.len()
    );
}

fn first_section(parsed: &TraceFile) -> usize {
    parsed
        .entries
        .iter()
        .find_map(|e| e.section)
        .expect("a checkpoint with a word section")
}

fn relabel(bytes: &mut [u8], entry: &trace::Entry) {
    let len = entry.blob.len();
    let set = trace::digest_set(&bytes[entry.at + 4..entry.at + 4 + len]);
    bytes[entry.at + 4 + len..entry.at + 68 + len].copy_from_slice(&set);
}

fn at(offset: usize) -> u64 {
    u64::try_from(offset).expect("fits")
}

/// Each planted defect is rejected, by the check that names it.
#[test]
fn check_trace_rejects_each_planted_defect_b3_09() {
    let small = Corpus::new(0xB3_09).bytes(64);
    let (word, word_parsed) = file(Mode::Hash, &small, 32, 1);
    let (bit, bit_parsed) = file(Mode::Hash, &small, 32, 2);
    let section = first_section(&word_parsed);
    let first_step = section + 89 + 16;
    let mut plants: Vec<(&str, Vec<u8>, &str, Option<u64>)> = Vec::new();

    let mut planted = word.clone();
    planted[first_step + 14] ^= 1;
    plants.push((
        "a flipped operation result",
        planted,
        "step",
        Some(at(first_step)),
    ));

    let mut planted = bit.clone();
    let bits = first_section(&bit_parsed) + 89 + 16 + STEP;
    planted[bits] ^= 1 << 4;
    plants.push(("a flipped carry bit", planted, "bit", Some(at(bits))));

    let mut planted = word.clone();
    let map = section + 89 + 16 + 112 * STEP;
    planted[map] ^= 1;
    plants.push((
        "a wrong permutation index",
        planted,
        "permutation",
        Some(at(map)),
    ));

    let mut planted = word.clone();
    let (a, b) = planted[first_step..first_step + 2 * STEP].split_at_mut(STEP);
    a.swap_with_slice(b);
    plants.push(("a reordered step", planted, "step", Some(at(first_step))));

    let input = Corpus::new(0xB3_09).bytes(2049);
    let (tree, tree_parsed) = file(Mode::Hash, &input, 32, 0);
    let checkpoint = &tree_parsed.entries[0];
    let mut planted = tree.clone();
    planted[checkpoint.at + 4 + 34 + 60] ^= 2;
    plants.push((
        "a wrong flag, label kept",
        planted.clone(),
        "digestSet",
        Some(at(checkpoint.at)),
    ));
    relabel(&mut planted, checkpoint);
    plants.push(("a wrong flag, relabelled", planted, "unknownLabel", None));

    let mut planted = tree.clone();
    planted[checkpoint.at + 4 + 98] ^= 1;
    plants.push((
        "a checkpoint whose message changed, label kept",
        planted,
        "digestSet",
        Some(at(checkpoint.at)),
    ));

    // The first parent's left child (chunk 0) swapped for its right (chunk
    // 1), its DigestSet recomputed so only the link itself is wrong.
    let parent = tree_parsed
        .entries
        .iter()
        .find(|e| e.kind() == 3 && e.blob[81] == 1)
        .expect("a non-root parent");
    let other = tree_parsed
        .entries
        .iter()
        .filter(|e| e.kind() == 2)
        .nth(1)
        .expect("a second chunk");
    let mut planted = tree.clone();
    planted[parent.at + 4 + 17..parent.at + 4 + 49].copy_from_slice(&other.digest_set[..32]);
    relabel(&mut planted, parent);
    plants.push(("a forged parent link", planted, "link", Some(at(parent.at))));

    let files: Vec<Vec<u8>> = plants.iter().map(|p| p.1.clone()).collect();
    for ((what, _, kind, offset), verdict) in plants.iter().zip(engine_check(&files)) {
        match verdict {
            Err(TraceRejection {
                kind: got,
                at: where_,
            }) => {
                assert_eq!(got, *kind, "{what}: rejected as {got} at {where_}");
                if let Some(offset) = offset {
                    assert_eq!(where_, *offset, "{what}");
                }
            }
            Ok(_) => panic!("{what} was accepted"),
        }
    }

    // A real trace of a different input is not this input's trace.
    let elsewhere = file(Mode::Hash, &small[1..], 32, 0).0;
    assert_eq!(
        check_against(Mode::Hash, &[&small], 32, &elsewhere),
        Err(TraceRejection {
            kind: "rootMismatch",
            at: 0
        })
    );
    let _ = (WORD_SECTION, BIT_SECTION);
}

/// The same (mode, input, outLen) gives byte-identical traces, however the
/// input is split into updates.
#[test]
fn a_trace_is_deterministic_and_split_invariant_b3_10() {
    let input = Corpus::new(0xB3_10).bytes(3000);
    for mode in modes() {
        for level in 0..2 {
            let whole = lex_trace_file(mode, &[&input], 77, level).expect("valid");
            assert_eq!(
                whole,
                lex_trace_file(mode, &[&input], 77, level).expect("valid")
            );
            let pieces: [&[u8]; 4] = [
                &input[..1],
                &input[1..1024],
                &input[1024..2049],
                &input[2049..],
            ];
            assert_eq!(
                whole,
                lex_trace_file(mode, &pieces, 77, level).expect("valid"),
                "{mode:?} level {level}"
            );
        }
    }
}

/// Identical sub-computations have one address: the first chunk of two
/// different inputs resolves to the same checkpoint and chunk labels.
#[test]
fn a_shared_chunk_has_one_address_b3_11() {
    let mut corpus = Corpus::new(0xB3_11);
    let shared = corpus.bytes(1024);
    let one = [shared.clone(), corpus.bytes(1000)].concat();
    let two = [shared, corpus.bytes(5000)].concat();
    let chunk0 = |input: &[u8]| {
        let (_, parsed) = file(Mode::Hash, input, 32, 0);
        let entries: Vec<String> = parsed
            .entries
            .iter()
            .take_while(|e| e.kind() != 2)
            .map(|e| e.label())
            .collect();
        let chunk = parsed
            .entries
            .iter()
            .find(|e| e.kind() == 2)
            .expect("a chunk")
            .label();
        (entries, chunk)
    };
    let (a, b) = (chunk0(&one), chunk0(&two));
    assert_eq!(a.0.len(), 16, "sixteen block checkpoints");
    assert_eq!(a, b, "the same chunk under one address");
    let (_, other) = file(Mode::Hash, &one, 32, 0);
    let (_, again) = file(Mode::Hash, &two, 32, 0);
    assert_ne!(
        trace::root(&other).digest_set,
        trace::root(&again).digest_set
    );
}
