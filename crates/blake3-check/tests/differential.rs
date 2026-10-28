//! `B3-02`, `B3-03`, `B3-04`: the exported packages against the `blake3`
//! crate on a seeded corpus, incremental hashing against one-shot hashing,
//! and the typed rejection of a key that is not 32 bytes.

use blake3_check::{
    lex_derive_key, lex_hash, lex_keyed_hash, lex_pieces, oracle, Corpus, Mode, BOUNDARY_LENGTHS,
    OUTPUT_LENGTHS,
};

fn context(corpus: &mut Corpus) -> Vec<u8> {
    let len = 1 + corpus.below(80);
    (0..len)
        .map(|_| b' ' + u8::try_from(corpus.below(95)).expect("below 95"))
        .collect()
}

/// Every boundary length, every mode, every output length, seeded bytes.
#[test]
fn the_blake3_crate_agrees_on_every_boundary_b3_02() {
    let mut corpus = Corpus::new(0xB3_02);
    let mut compared = 0;
    for &len in BOUNDARY_LENGTHS {
        let message = corpus.bytes(len);
        let key = corpus.bytes(32);
        let context = context(&mut corpus);
        for &out in OUTPUT_LENGTHS {
            assert_eq!(
                lex_hash(&message, out),
                oracle(Mode::Hash, &message, out),
                "hash, len {len}, out {out}"
            );
            assert_eq!(
                lex_keyed_hash(&key, &message, out),
                Ok(oracle(Mode::Keyed(&key), &message, out)),
                "keyed_hash, len {len}, out {out}"
            );
            assert_eq!(
                lex_derive_key(&context, &message, out),
                oracle(Mode::Derive(&context), &message, out),
                "derive_key, len {len}, out {out}"
            );
            compared += 3;
        }
    }
    eprintln!(
        "B3-02: {compared} outputs equal the blake3 crate over {} lengths",
        BOUNDARY_LENGTHS.len()
    );
}

/// The differential can fail: a message one bit away hashes differently.
#[test]
fn a_one_bit_change_is_seen_by_the_differential_b3_02() {
    let mut message = Corpus::new(7).bytes(1025);
    let before = lex_hash(&message, 32);
    message[1024] ^= 0x80;
    assert_ne!(lex_hash(&message, 32), before);
    assert_eq!(lex_hash(&message, 32), oracle(Mode::Hash, &message, 32));
}

/// Any split of the input into update pieces gives the one-shot digest, in
/// every mode: empty pieces, single bytes, pieces that end exactly on a chunk
/// edge, and seeded random cuts.
#[test]
fn split_updates_equal_one_shot_hashing_b3_03() {
    let mut corpus = Corpus::new(0xB3_03);
    let mut splits = 0;
    for &len in BOUNDARY_LENGTHS {
        let message = corpus.bytes(len);
        let key = corpus.bytes(32);
        let context = context(&mut corpus);
        let mut cuts: Vec<Vec<usize>> = vec![
            vec![],
            vec![0, len],
            (0..=len).step_by(1024).collect(),
            (0..=len).step_by(64).collect(),
        ];
        if len <= 2048 {
            cuts.push((0..=len).collect());
        }
        for _ in 0..4 {
            let mut random: Vec<usize> = (0..1 + corpus.below(8))
                .map(|_| corpus.below(len + 1))
                .collect();
            random.sort_unstable();
            cuts.push(random);
        }
        for cut in &cuts {
            let mut edges = vec![0];
            edges.extend(cut.iter().copied());
            edges.push(len);
            let pieces: Vec<&[u8]> = edges.windows(2).map(|w| &message[w[0]..w[1]]).collect();
            for mode in [Mode::Hash, Mode::Keyed(&key), Mode::Derive(&context)] {
                let one_shot = oracle(mode, &message, 131);
                assert_eq!(
                    lex_pieces(mode, &pieces, 131),
                    Ok(one_shot),
                    "{mode:?}, len {len}, {} pieces",
                    pieces.len()
                );
                splits += 1;
            }
        }
    }
    let empty: [&[u8]; 0] = [];
    assert_eq!(
        lex_pieces(Mode::Hash, &empty, 32),
        Ok(oracle(Mode::Hash, b"", 32)),
        "no update at all is the empty message"
    );
    eprintln!("B3-03: {splits} split inputs equal their one-shot digest");
}

/// A key that is not 32 bytes is `Blake3Error.keyLength n`, in both roots
/// that take a key; 32 bytes is accepted.
#[test]
fn a_key_that_is_not_32_bytes_is_rejected_b3_04() {
    let message = b"abc";
    for len in [0usize, 1, 16, 31, 33, 64] {
        let key = vec![7u8; len];
        let expected = Err(u64::try_from(len).expect("fits"));
        assert_eq!(
            lex_keyed_hash(&key, message, 32),
            expected,
            "keyedHash, key {len}"
        );
        assert_eq!(
            lex_pieces(Mode::Keyed(&key), &[message], 32),
            expected,
            "hashPieces, key {len}"
        );
    }
    let key = [7u8; 32];
    assert_eq!(
        lex_keyed_hash(&key, message, 32),
        Ok(oracle(Mode::Keyed(&key), message, 32))
    );
}
