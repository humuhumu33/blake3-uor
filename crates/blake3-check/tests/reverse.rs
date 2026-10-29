//! `B3-13` to `B3-16`: phase 1. The step machine run backwards, the cursor,
//! seek, and navigation by label. The reverse roots run in the committed
//! Prism image's engine (`docs/trace.md`); navigation is the independent
//! trace reader's.

use blake3_check::trace::{self, TraceFile, STEP};
use blake3_check::value::{adt, as_bytes, boolean, bytes, fields, list, nat, word};
use blake3_check::{engine_calls, lex_trace_file, Corpus, Mode};

fn tree(input: &[u8], out_len: usize) -> TraceFile {
    trace::parse(&lex_trace_file(Mode::Hash, &[input], out_len, 0).expect("hash mode"))
}

fn checkpoint<'a>(file: &'a TraceFile, label: &[u8]) -> &'a trace::Entry {
    file.entries
        .iter()
        .find(|e| e.kind() == 1 && &e.digest_set[..32] == label)
        .expect("a checkpoint with that label")
}

/// A root output block (all 16 words) runs back, with its message and CV, to
/// exactly its checkpoint's initial state; with only 8 words kept (a
/// chaining value) it does not.
#[test]
fn a_compression_runs_back_to_its_start_b3_13() {
    let mut corpus = Corpus::new(0xB3_13);
    let mut calls = Vec::new();
    let mut expected = Vec::new();
    for len in [0usize, 3, 64, 1025, 2049] {
        let file = tree(&corpus.bytes(len), 200);
        for output in file.entries.iter().filter(|e| e.kind() == 4) {
            let cp = checkpoint(&file, &output.blob[9..41]);
            let state = &cp.blob[34..98];
            let message = &cp.blob[98..162];
            let out = &output.blob[41..105];
            calls.push((
                "unmix",
                serde_json::json!([bytes(&state[..32]), bytes(message), bytes(out)]),
            ));
            expected.push((state.to_vec(), true));
            let mut truncated = out.to_vec();
            truncated[32..].fill(0);
            calls.push((
                "unmix",
                serde_json::json!([bytes(&state[..32]), bytes(message), bytes(&truncated)]),
            ));
            expected.push((state.to_vec(), false));
        }
    }
    let got = engine_calls(&calls);
    for (k, (value, (state, equal))) in got.iter().zip(&expected).enumerate() {
        assert_eq!(as_bytes(value) == *state, *equal, "call {k}");
    }
    eprintln!(
        "B3-13: {} root output blocks ran back to their checkpoints; their truncations did not",
        got.len() / 2
    );
}

/// Every operation's inverse undoes it: subtraction for add and addMsg, the
/// same xor for xor and xorCv, rotate-left for rotate-right, over edge values
/// and a seeded corpus.
#[test]
fn every_operation_steps_back_b3_14() {
    let edges = [
        0u32,
        1,
        2,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
    ];
    let mut corpus = Corpus::new(0xB3_14);
    let mut calls = Vec::new();
    for kind in [0u64, 1, 2, 4] {
        for &a in &edges {
            for &b in &edges {
                calls.push((
                    "stepRoundTrip",
                    serde_json::json!([nat(kind), word(a), word(b)]),
                ));
            }
        }
        for _ in 0..300 {
            let (a, b) = (corpus.next_u64() as u32, corpus.next_u64() as u32);
            calls.push((
                "stepRoundTrip",
                serde_json::json!([nat(kind), word(a), word(b)]),
            ));
        }
    }
    for amount in [7u32, 8, 12, 16] {
        for &a in &edges {
            calls.push((
                "stepRoundTrip",
                serde_json::json!([nat(3), word(a), word(amount)]),
            ));
        }
        for _ in 0..300 {
            calls.push((
                "stepRoundTrip",
                serde_json::json!([nat(3), word(corpus.next_u64() as u32), word(amount)]),
            ));
        }
    }
    let got = engine_calls(&calls);
    assert!(
        got.iter().all(|v| v["value"] == true),
        "every operation round-trips"
    );
    eprintln!("B3-14: {} operations stepped forward and back", got.len());
}

/// A cursor move: from (node, step, bit), by a granularity, backwards or
/// not, to where it should land.
type Move = ((u64, u64, u64), u64, bool, (u64, u64, u64));

fn address(node: u64, step: u64, bit: u64) -> serde_json::Value {
    adt(0, vec![nat(node), nat(step), nat(bit)])
}

/// Moving back by stepBack agrees with replaying forward from the
/// checkpoint at every step; the cursor moves by bit, op, G, round and
/// compression and stays at the ends; and seek returns the same step record
/// the word-level trace holds, with its reversibility tag.
#[test]
fn back_replay_cursor_and_seek_agree_b3_15() {
    let input = Corpus::new(0xB3_15).bytes(1025);
    let file = tree(&input, 64);
    let mut calls = Vec::new();
    for cp in file.entries.iter().filter(|e| e.kind() == 1).step_by(5) {
        let s = &cp.blob[34..98];
        let w = |i: usize| u32::from_le_bytes(s[4 * i..4 * i + 4].try_into().expect("4 bytes"));
        calls.push((
            "prevAgrees",
            serde_json::json!([
                bytes(&s[..32]),
                bytes(&cp.blob[98..162]),
                word(w(12)),
                word(w(13)),
                word(w(14)),
                word(w(15))
            ]),
        ));
    }
    let agreements = calls.len();
    let moves: Vec<Move> = vec![
        ((0, 13, 31), 0, false, (0, 14, 0)),
        ((0, 14, 0), 0, true, (0, 13, 31)),
        ((0, 0, 0), 0, true, (0, 0, 0)),
        ((0, 799, 0), 1, false, (1, 0, 0)),
        ((1, 0, 0), 1, true, (0, 799, 0)),
        ((0, 15, 0), 2, false, (0, 28, 0)),
        ((0, 15, 0), 2, true, (0, 14, 0)),
        ((0, 14, 0), 2, true, (0, 0, 0)),
        ((0, 784, 0), 2, true, (0, 770, 0)),
        ((0, 100, 0), 3, false, (0, 112, 0)),
        ((0, 700, 0), 3, false, (0, 784, 0)),
        ((0, 784, 0), 3, false, (1, 0, 0)),
        ((0, 784, 0), 3, true, (0, 672, 0)),
        ((2, 5, 0), 4, true, (1, 0, 0)),
        ((2, 5, 0), 4, false, (2, 5, 0)),
    ];
    for ((n, s, b), g, back, _) in &moves {
        calls.push((
            "cursorMove",
            serde_json::json!([address(*n, *s, *b), nat(*g), boolean(*back), nat(3)]),
        ));
    }
    let word_file = lex_trace_file(Mode::Hash, &[&input], 64, 1).expect("hash mode");
    let parsed = trace::parse(&word_file);
    let sections: Vec<usize> = parsed.entries.iter().filter_map(|e| e.section).collect();
    let samples = [
        (0usize, 0usize),
        (0, 1),
        (3, 500),
        (16, 783),
        (16, 784),
        (16, 799),
        (17, 799),
    ];
    for (node, step) in samples {
        calls.push((
            "seekStep",
            serde_json::json!([
                adt(0, vec![]),
                list(vec![bytes(&input)]),
                nat(64),
                nat(node as u64),
                nat(step as u64)
            ]),
        ));
    }
    let got = engine_calls(&calls);
    assert!(
        got[..agreements].iter().all(|v| v["value"] == true),
        "back and replay agree at every step"
    );
    for (k, (_, _, _, want)) in moves.iter().enumerate() {
        assert_eq!(
            fields(&got[agreements + k]),
            vec![want.0, want.1, want.2],
            "move {k}"
        );
    }
    for (k, (node, step)) in samples.iter().enumerate() {
        let record = as_bytes(&got[agreements + moves.len() + k]["value"]);
        let section = sections[*node];
        // A round's 16-byte permutation map precedes its first step.
        let maps = if *step < 784 { step / 112 + 1 } else { 7 };
        let offset = section + 89 + maps * 16 + step * STEP;
        assert_eq!(
            &record[..STEP],
            &word_file[offset..offset + STEP],
            "seek ({node}, {step})"
        );
        // The feed-forward of a chunk's last block keeps 8 words: lossy.
        let root_output = parsed
            .entries
            .iter()
            .filter(|e| e.kind() == 1)
            .nth(*node)
            .expect("node")
            .blob[1]
            == 2;
        let want_tag = u8::from(*step >= 784 && !root_output);
        assert_eq!(record[STEP], want_tag, "tag ({node}, {step})");
    }
    eprintln!(
        "B3-15: {agreements} compressions agree step by step; {} moves; {} seeks",
        moves.len(),
        samples.len()
    );
}

/// From any label, the trace's links lead back to what produced it and
/// forward to what consumes it.
#[test]
fn labels_navigate_to_producers_and_consumers_b3_16() {
    let file = tree(&Corpus::new(0xB316).bytes(3073), 32);
    let root = trace::root(&file);
    let label = |e: &trace::Entry| -> [u8; 32] { e.digest_set[..32].try_into().expect("32 bytes") };
    // Backwards from the root: its tree node is a parent whose children are
    // a parent and a chunk, down to the checkpoints of chunk 0.
    let node =
        trace::producer(&file, &trace::references(&root.blob)[0]).expect("the root's tree node");
    assert_eq!(node.kind(), 3, "four chunks: the root is a parent");
    let left = trace::producer(&file, &trace::references(&node.blob)[0]).expect("left child");
    assert_eq!(left.kind(), 3, "chunks 0 and 1 merged");
    let chunk0 = trace::producer(&file, &trace::references(&left.blob)[0]).expect("chunk 0");
    assert_eq!(chunk0.kind(), 2);
    let first =
        trace::producer(&file, &trace::references(&chunk0.blob)[0]).expect("chunk 0's first block");
    assert_eq!(first.kind(), 1);
    assert_eq!(
        u64::from_le_bytes(first.blob[2..10].try_into().expect("8")),
        0,
        "chunk 0"
    );
    // Forwards from that checkpoint: the chunk that lists it and the events.
    let consumers: Vec<u8> = trace::consumers(&file, &label(first))
        .iter()
        .map(|e| e.kind())
        .collect();
    assert_eq!(consumers, vec![2, 5]);
    // The root's output blocks lead to their root checkpoints.
    for output in trace::references(&root.blob)
        .iter()
        .skip(2)
        .filter_map(|l| trace::producer(&file, l))
        .filter(|e| e.kind() == 4)
    {
        let cp = trace::producer(&file, &trace::references(&output.blob)[0]).expect("a checkpoint");
        assert_eq!(cp.blob[1], 2, "a root output checkpoint");
    }
}
