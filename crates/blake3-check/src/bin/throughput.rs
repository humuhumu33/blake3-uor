//! Throughput of the exported package against the `blake3` crate, measured
//! and reported, never asserted (`OPEN-B3-THROUGHPUT`).
//!
//! `cargo run --release -p blake3-check --bin throughput` measures the crate's
//! default build (runtime-selected SIMD, single thread);
//! `--features portable` measures its portable build. Both print the export.

use std::hint::black_box;
use std::time::{Duration, Instant};

use blake3_check::{lex_hash, oracle, Corpus, Mode};

/// Mean MiB/s of `hash` over `message`, repeated until a second has passed.
fn rate(message: &[u8], hash: &dyn Fn(&[u8]) -> Vec<u8>) -> f64 {
    let mut rounds = 0u32;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) || rounds < 3 {
        black_box(hash(black_box(message)));
        rounds += 1;
    }
    let bytes = f64::from(rounds) * message.len() as f64;
    bytes / start.elapsed().as_secs_f64() / f64::from(1u32 << 20)
}

fn main() {
    let build = if cfg!(feature = "portable") {
        "portable"
    } else {
        "default (SIMD)"
    };
    println!("| input | LexLean export MiB/s | blake3 crate {build} MiB/s | ratio |");
    println!("| --- | --- | --- | --- |");
    let mut corpus = Corpus::new(0x7468_726f);
    for len in [64usize, 1024, 16 << 10, 1 << 20] {
        let message = corpus.bytes(len);
        assert_eq!(
            lex_hash(&message, 32),
            oracle(Mode::Hash, &message, 32),
            "a measurement of a wrong digest is no measurement"
        );
        let export = rate(&message, &|m| lex_hash(m, 32));
        let crate_rate = rate(&message, &|m| oracle(Mode::Hash, m, 32));
        println!(
            "| {len} B | {export:.2} | {crate_rate:.1} | {:.0}x |",
            crate_rate / export
        );
    }
}
