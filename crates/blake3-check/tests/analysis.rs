//! `B3-18` and `B3-19`: phase 2's difference traces and dependency map of
//! one compression. The analysis roots run in the committed Prism image's
//! engine (`docs/trace.md`). What they compute is held to two independent
//! sources: the certified word-level trace files (`traceFile`), whose step
//! records give the observed differences of every message and CV bit, and a
//! taint propagation written here from the BLAKE3 specification.

use blake3_check::trace::{self, STEP};
use blake3_check::value::{adt, as_bytes, bytes, nat, word};
use blake3_check::{engine_calls_parallel, lex_trace_file, Corpus, Mode};

/// Steps of one compression: 7 rounds of 112, then the 16 feed-forward.
const STEPS: usize = 800;
const ROUND_STEPS: usize = 7 * 112;
/// A taint set: 1024 bits, message words 0-15 then initial state words 0-15.
type Set = [u32; 32];
/// The taint map's bytes per step: 32 sets of 128 bytes.
const MAP_STEP: usize = 32 * 128;

// The BLAKE3 specification's G lanes, rotations and message permutation,
// written out here rather than taken from the generator.
const LANES: [[usize; 4]; 8] = [
    [0, 4, 8, 12],
    [1, 5, 9, 13],
    [2, 6, 10, 14],
    [3, 7, 11, 15],
    [0, 5, 10, 15],
    [1, 6, 11, 12],
    [2, 7, 8, 13],
    [3, 4, 9, 14],
];
const PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

/// The base compression: the root block of a 64-byte keyed hash (CV = key,
/// counter 0, length 64, flags CHUNK_START | CHUNK_END | ROOT | KEYED_HASH).
struct Base {
    key: [u8; 32],
    block: [u8; 64],
}

const FLAGS: u32 = 1 | 2 | 8 | 16;

impl Base {
    fn new() -> Self {
        let mut corpus = Corpus::new(0xB3_18);
        Self {
            key: corpus.bytes(32).try_into().expect("32"),
            block: corpus.bytes(64).try_into().expect("64"),
        }
    }

    /// The compression's input with input bit `bit` flipped (message bits
    /// 0-511; then the initial state's, 512 + 32w + i): (key, block,
    /// counter low, counter high, length, flags).
    fn flipped(&self, bit: Option<usize>) -> ([u8; 32], [u8; 64], [u32; 4]) {
        let (mut key, mut block, mut words) = (self.key, self.block, [0, 0, 64, FLAGS]);
        match bit {
            None => {}
            Some(b) if b < 512 => block[b / 8] ^= 1 << (b % 8),
            Some(b) if b < 768 => key[(b - 512) / 8] ^= 1 << (b % 8),
            Some(b) => {
                let w = (b - 512) / 32;
                assert!(w >= 12, "IV bits are constants, not inputs");
                words[w - 12] ^= 1 << (b % 32);
            }
        }
        (key, block, words)
    }

    fn engine_input(&self, bit: Option<usize>) -> serde_json::Value {
        let (key, block, w) = self.flipped(bit);
        adt(
            0,
            vec![
                bytes(&key),
                bytes(&block),
                word(w[0]),
                word(w[1]),
                word(w[2]),
                word(w[3]),
            ],
        )
    }

    /// The 800 step records of the input with `bit` flipped (a message or
    /// key bit), read from the certified word-level trace file.
    fn records(&self, bit: Option<usize>) -> Vec<[u8; STEP]> {
        let (key, block, words) = self.flipped(bit);
        assert_eq!(
            words,
            [0, 0, 64, FLAGS],
            "the trace file reaches message and key bits"
        );
        let file = lex_trace_file(Mode::Keyed(&key), &[&block], 32, 1).expect("a 32-byte key");
        let parsed = trace::parse(&file);
        let section = parsed
            .entries
            .iter()
            .find_map(|e| e.section)
            .expect("one checkpoint");
        (0..STEPS)
            .map(|k| {
                // A round's 16-byte permutation map precedes its first step.
                let maps = if k < ROUND_STEPS { k / 112 + 1 } else { 7 };
                let at = section + 89 + maps * 16 + k * STEP;
                file[at..at + STEP].try_into().expect("a step record")
            })
            .collect()
    }
}

/// The difference trace of two runs' step records: per step, the xor of
/// their left operands, right operands and results.
fn observed(a: &[[u8; STEP]], b: &[[u8; STEP]]) -> Vec<u8> {
    a.iter()
        .zip(b)
        .flat_map(|(x, y)| (6..STEP).map(move |i| x[i] ^ y[i]))
        .collect()
}

fn result_diff(diff: &[u8], k: usize) -> u32 {
    u32::from_le_bytes(diff[k * 12 + 8..k * 12 + 12].try_into().expect("4"))
}

/// Destination of every step, from the specification.
fn destinations() -> Vec<usize> {
    let mut dest = Vec::with_capacity(STEPS);
    for _ in 0..7 {
        for [a, b, c, d] in LANES {
            dest.extend([a, a, d, d, c, b, b, a, a, d, d, c, b, b]);
        }
    }
    dest.extend(0..16);
    dest
}

/// For each state word, the first step after which the runs differ in it:
/// 0 when the initial states already do, 800 when never.
fn reach(diff: &[u8], initial: &[bool; 16], dest: &[usize]) -> [usize; 16] {
    let mut reach = initial.map(|d| if d { 0 } else { STEPS });
    for k in 0..ROUND_STEPS {
        if result_diff(diff, k) != 0 && reach[dest[k]] == STEPS {
            reach[dest[k]] = k;
        }
    }
    reach
}

/// Which initial state words input bit `bit` belongs to.
fn initial_diff(bit: usize) -> [bool; 16] {
    let mut d = [false; 16];
    if bit >= 512 {
        d[(bit - 512) / 32] = true;
    }
    d
}

/// One step's taint rule: the 16 state words' sets to its result's.
type StepRule<'a> = dyn Fn(&[[Set; 32]; 16]) -> [Set; 32] + 'a;

/// The dependency map by the specification, independently of the model:
/// structural taint with sum bit i depending on bits 0..i of both operands,
/// xor bit i on bit i of both, right-rotation by n bit i on bit i+n.
fn specification_map() -> Vec<u8> {
    let single = |word: usize, bit: usize| {
        let mut s: Set = [0; 32];
        s[word] = 1 << bit;
        s
    };
    let input_word = |word: usize| -> [Set; 32] { std::array::from_fn(|i| single(word, i)) };
    let mut state: [[Set; 32]; 16] = std::array::from_fn(|w| {
        if (8..12).contains(&w) {
            [[0; 32]; 32]
        } else {
            input_word(16 + w)
        }
    });
    let mut map = Vec::with_capacity(STEPS * MAP_STEP);
    let mut emit = |result: &[Set; 32]| {
        for set in result {
            for w in set {
                map.extend(w.to_le_bytes());
            }
        }
    };
    let union = |a: &Set, b: &Set| -> Set { std::array::from_fn(|i| a[i] | b[i]) };
    let add = |l: &[Set; 32], r: &[Set; 32]| -> [Set; 32] {
        let mut carried: Set = [0; 32];
        std::array::from_fn(|i| {
            carried = union(&carried, &union(&l[i], &r[i]));
            carried
        })
    };
    let xor = |l: &[Set; 32], r: &[Set; 32]| -> [Set; 32] {
        std::array::from_fn(|i| union(&l[i], &r[i]))
    };
    let ror = |l: &[Set; 32], n: usize| -> [Set; 32] { std::array::from_fn(|i| l[(i + n) % 32]) };
    let mut schedule: [usize; 16] = std::array::from_fn(|j| j);
    for _ in 0..7 {
        for (g, [a, b, c, d]) in LANES.into_iter().enumerate() {
            let (mx, my) = (input_word(schedule[2 * g]), input_word(schedule[2 * g + 1]));
            for (rot1, rot2, m) in [(16, 12, &mx), (8, 7, &my)] {
                // Each step reads the state its predecessor wrote.
                let steps: [(usize, &StepRule<'_>); 7] = [
                    (a, &|s| add(&s[a], &s[b])),
                    (a, &|s| add(&s[a], m)),
                    (d, &|s| xor(&s[d], &s[a])),
                    (d, &|s| ror(&s[d], rot1)),
                    (c, &|s| add(&s[c], &s[d])),
                    (b, &|s| xor(&s[b], &s[c])),
                    (b, &|s| ror(&s[b], rot2)),
                ];
                for (dest, step) in steps {
                    let value = step(&state);
                    emit(&value);
                    state[dest] = value;
                }
            }
        }
        schedule = std::array::from_fn(|j| schedule[PERMUTATION[j]]);
    }
    // The feed-forward: out[i] = v[i] ^ v[i+8]; out[i+8] = v[i+8] ^ cv[i].
    for i in 0..8 {
        emit(&xor(&state[i], &state[i + 8]));
    }
    for i in 0..8 {
        emit(&xor(&state[i + 8], &input_word(16 + i)));
    }
    map
}

fn taint_has(map: &[u8], k: usize, bit_of_result: usize, input: usize) -> bool {
    let at = k * MAP_STEP + bit_of_result * 128 + input / 8;
    map[at] >> (input % 8) & 1 == 1
}

/// Every input bit that is not an IV constant.
fn inputs() -> impl Iterator<Item = usize> {
    (0..768).chain(896..1024)
}

/// Two inputs differing by one bit: the difference at every step, forwards
/// and backwards, and the step at which it reaches each state word.
#[test]
fn difference_traces_b3_18() {
    let base = Base::new();
    let dest = destinations();
    let a = base.records(None);
    // Observed differences of all 768 message and key bits, from the
    // certified trace files.
    let observed_all: Vec<(usize, Vec<u8>)> = (0..768)
        .map(|bit| (bit, observed(&a, &base.records(Some(bit)))))
        .collect();
    // The engine's roots on a sample: message bits, key bits, and the
    // counter, length and flags bits the trace files cannot reach.
    let sample = [
        0usize, 31, 32, 255, 300, 511, 512, 600, 767, 896, 927, 928, 959, 960, 991, 992, 1023,
    ];
    let mut calls = Vec::new();
    for &bit in &sample {
        let args = serde_json::json!([base.engine_input(None), base.engine_input(Some(bit))]);
        calls.push(("diffTrace", args.clone()));
        calls.push(("diffTraceBack", args.clone()));
        calls.push(("diffReach", args));
    }
    let got = engine_calls_parallel(&calls, 2);
    for (n, &bit) in sample.iter().enumerate() {
        let forward = as_bytes(&got[3 * n]);
        let backward = as_bytes(&got[3 * n + 1]);
        let reached: Vec<usize> = got[3 * n + 2]["items"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|v| v["value"].as_str().expect("nat").parse().expect("usize"))
            .collect();
        assert_eq!(forward.len(), STEPS * 12, "bit {bit}");
        assert_eq!(backward, forward, "bit {bit}: backwards equals forwards");
        if bit < 768 {
            assert_eq!(
                forward, observed_all[bit].1,
                "bit {bit}: the engine equals the trace files"
            );
        }
        assert_eq!(
            reached,
            reach(&forward, &initial_diff(bit), &dest).to_vec(),
            "bit {bit}: reach"
        );
    }
    // Reach of message bit 0, and the latest reach of each word over every
    // message and key bit.
    let first = reach(&observed_all[0].1, &initial_diff(0), &dest);
    let mut latest = [0usize; 16];
    for (bit, diff) in &observed_all {
        for (w, r) in reach(diff, &initial_diff(*bit), &dest)
            .into_iter()
            .enumerate()
        {
            latest[w] = latest[w].max(r);
        }
    }
    let show = |r: &[usize; 16]| {
        r.iter()
            .enumerate()
            .map(|(w, &k)| format!("v{w} {k} (r{})", k / 112))
            .collect::<Vec<_>>()
            .join(", ")
    };
    eprintln!(
        "B3-18: {} flips observed, {} in the engine forwards and backwards",
        observed_all.len(),
        sample.len()
    );
    eprintln!("B3-18: message bit 0 reaches {}", show(&first));
    eprintln!("B3-18: latest over 768 bits {}", show(&latest));
    // The share of differing state bits after each round, over the 768.
    let mut shares = Vec::new();
    for r in 0..7 {
        let mut ones = 0u64;
        for (bit, diff) in &observed_all {
            let mut state = [0u32; 16];
            if *bit >= 512 {
                state[(bit - 512) / 32] = 1 << (bit % 32);
            }
            for k in 0..112 * (r + 1) {
                state[dest[k]] = result_diff(diff, k);
            }
            ones += state.iter().map(|w| u64::from(w.count_ones())).sum::<u64>();
        }
        shares.push(format!("r{r} {:.3}", ones as f64 / (768.0 * 512.0)));
    }
    eprintln!(
        "B3-18: differing state bits after each round {}",
        shares.join(", ")
    );
}

/// The tree level: one flipped input bit changes exactly the compressions
/// on its chunk's slice path, from the flipped block on.
#[test]
fn a_difference_follows_the_slice_path_b3_18() {
    let mut corpus = Corpus::new(0xB3_18);
    let input = corpus.bytes(4097);
    let mut flipped = input.clone();
    flipped[2048 + 5 * 64 + 3] ^= 0x10;
    let tree =
        |data: &[u8]| trace::parse(&lex_trace_file(Mode::Hash, &[data], 32, 0).expect("hash"));
    let (x, y) = (tree(&input), tree(&flipped));
    let checkpoints = |t: &trace::TraceFile| -> Vec<Vec<u8>> {
        t.entries
            .iter()
            .filter(|e| e.kind() == 1)
            .map(|e| e.blob.clone())
            .collect()
    };
    let (cx, cy) = (checkpoints(&x), checkpoints(&y));
    assert_eq!(cx.len(), cy.len(), "the same tree");
    // A checkpoint's initial state sits at blob[34..98]: word 12 the counter,
    // word 15 the flags.
    let word = |blob: &[u8], w: usize| {
        u32::from_le_bytes(blob[34 + 4 * w..38 + 4 * w].try_into().expect("4"))
    };
    let differing: Vec<&Vec<u8>> = cx
        .iter()
        .zip(&cy)
        .filter(|(p, q)| p != q)
        .map(|(p, _)| p)
        .collect();
    let blocks: Vec<_> = differing.iter().filter(|b| word(b, 15) & 4 == 0).collect();
    let parents = differing.len() - blocks.len();
    assert!(
        blocks.iter().all(|b| word(b, 12) == 2),
        "only chunk 2's blocks"
    );
    assert_eq!(blocks.len(), 16 - 5, "chunk 2 from block 5 on");
    // 5 chunks: root (4 | 1), then (2 | 2), then (1 | 1): chunk 2's path has 3.
    assert_eq!(parents, 3, "the parents on chunk 2's path");
    eprintln!(
        "B3-18: a bit of chunk 2, block 5 of a 4097-byte input changes {} of {} compressions: {} blocks and the {} parents of its slice path",
        differing.len(),
        cx.len(),
        blocks.len(),
        parents
    );
}

/// The dependency map: per bit per step, the input bits with a structural
/// path to it, against the specification and against every observed
/// difference.
#[test]
fn dependency_map_b3_19() {
    let mut calls: Vec<(&str, serde_json::Value)> = (0..8)
        .map(|r| ("taintRound", serde_json::json!([nat(r)])))
        .collect();
    calls.push(("fullTaintSteps", serde_json::json!([])));
    calls.push(("wordDiffusion", serde_json::json!([])));
    let got = engine_calls_parallel(&calls, 2);
    let map: Vec<u8> = got[..8].iter().flat_map(as_bytes).collect();
    assert_eq!(map.len(), STEPS * MAP_STEP, "32 sets of 128 bytes per step");
    let spec = specification_map();
    assert_eq!(map, spec, "the model's map equals the specification's");

    // Fully diffused: every input bit (all but the 128 IV bits) in a set.
    let full = |k: usize, i: usize| inputs().all(|b| taint_has(&map, k, i, b));
    let dest = destinations();
    let mut first_full = [STEPS; 16];
    for k in 0..ROUND_STEPS {
        if first_full[dest[k]] == STEPS && (0..32).all(|i| full(k, i)) {
            first_full[dest[k]] = k;
        }
    }
    let engine_full: Vec<usize> = got[8]["items"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|v| v["value"].as_str().expect("nat").parse().expect("usize"))
        .collect();
    assert_eq!(engine_full, first_full.to_vec(), "fullTaintSteps");
    let bit_level = *first_full.iter().max().expect("16 words");
    let word_level: usize = got[9]["value"]
        .as_str()
        .expect("nat")
        .parse()
        .expect("usize");

    // Every observed difference lies inside the map: no flipped bit ever
    // changes a result bit whose set lacks it.
    let base = Base::new();
    let a = base.records(None);
    let mut checked = 0u64;
    let (mut equal, mut later, mut gap) = (0u32, 0u32, 0usize);
    let mut observed_full = 0usize;
    for bit in 0..768 {
        let diff = observed(&a, &base.records(Some(bit)));
        for k in 0..STEPS {
            let d = result_diff(&diff, k);
            for i in 0..32 {
                if d >> i & 1 == 1 {
                    assert!(
                        taint_has(&map, k, i, bit),
                        "bit {bit} changed step {k} bit {i} outside its set"
                    );
                    checked += 1;
                }
            }
        }
        // Structural reach: the first step whose result's sets hold the bit.
        let observed_reach = reach(&diff, &initial_diff(bit), &dest);
        let mut structural = initial_diff(bit).map(|d| if d { 0 } else { STEPS });
        for k in 0..ROUND_STEPS {
            if structural[dest[k]] == STEPS && (0..32).any(|i| taint_has(&map, k, i, bit)) {
                structural[dest[k]] = k;
            }
        }
        for w in 0..16 {
            assert!(
                observed_reach[w] >= structural[w],
                "bit {bit} word {w}: observed before possible"
            );
            if observed_reach[w] == structural[w] {
                equal += 1;
            } else {
                later += 1;
                gap = gap.max(observed_reach[w] - structural[w]);
            }
        }
        observed_full = observed_full.max(*observed_reach.iter().max().expect("16"));
    }
    eprintln!(
        "B3-19: map {} bytes, equal to the specification's; blake3 {}",
        map.len(),
        blake3::hash(&map)
    );
    eprintln!(
        "B3-19: full diffusion, every bit of every word on every input bit: step {bit_level} (round {}); word level: step {word_level} (round {}); per word {first_full:?}",
        bit_level / 112,
        word_level / 112
    );
    eprintln!(
        "B3-19: {checked} observed differing bits all inside the map; word reach observed = structural {equal} times, later {later} times (at most {gap} steps); every word differs for every flip by step {observed_full} (round {})",
        observed_full / 112
    );
}
