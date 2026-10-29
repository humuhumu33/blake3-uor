//! Trace sizes per level, measured and reported, never asserted.
//!
//! `cargo run --release -p blake3-check --bin trace_sizes` writes each level
//! of the trace for 64 B and 1 KiB, and the tree level for 1 MiB. A word or
//! bit section has a fixed size per checkpoint, so the 1 MiB word and bit
//! sizes are the tree size plus that many sections; the same arithmetic is
//! checked against the files written for the smaller inputs.

use std::time::Instant;

use blake3_check::trace::{self, BIT_SECTION, WORD_SECTION};
use blake3_check::{lex_trace_file, Corpus, Mode};

fn main() {
    println!("| input | checkpoints | tree level | word level | bit level |");
    println!("| --- | --- | --- | --- | --- |");
    let mut corpus = Corpus::new(0x5122);
    for len in [64usize, 1024, 1 << 20] {
        let input = corpus.bytes(len);
        let start = Instant::now();
        let tree = lex_trace_file(Mode::Hash, &[&input], 32, 0).expect("hash mode");
        let seconds = start.elapsed().as_secs_f64();
        let checkpoints = trace::parse(&tree)
            .entries
            .iter()
            .filter(|e| e.kind() == 1)
            .count();
        let word = tree.len() + checkpoints * WORD_SECTION;
        let bit = tree.len() + checkpoints * BIT_SECTION;
        let note = if len <= 1024 {
            let written = |level| {
                lex_trace_file(Mode::Hash, &[&input], 32, level)
                    .expect("hash mode")
                    .len()
            };
            assert_eq!(
                written(1),
                word,
                "the word level is the tree plus one section per checkpoint"
            );
            assert_eq!(
                written(2),
                bit,
                "the bit level is the tree plus one section per checkpoint"
            );
            "written"
        } else {
            "computed"
        };
        println!(
            "| {len} B | {checkpoints} | {} B ({seconds:.1} s) | {word} B ({note}) | {bit} B ({note}) |",
            tree.len()
        );
    }
}
