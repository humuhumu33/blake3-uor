//! `B3-01`: the exported packages reproduce every official BLAKE3 vector.

use blake3_check::{
    lex_derive_key, lex_hash, lex_hash_hex, lex_keyed_hash, lex_pieces, official_cases,
    official_input, root, to_hex, Mode, OFFICIAL_CONTEXT, OFFICIAL_KEY,
};

/// Every case, every mode, the full 131-byte extended output, plus the
/// default 32-byte digest through the hex root and the incremental root.
#[test]
fn every_official_vector_in_every_mode_b3_01() {
    let cases = official_cases(&root().join("oracles/blake3-1.8.7/test_vectors.json"));
    assert_eq!(cases.len(), 35, "the 1.8.7 file has 35 cases");
    let mut checked = 0;
    for case in &cases {
        let input = official_input(case.input_len);
        let n = case.input_len;
        assert_eq!(lex_hash(&input, 131), case.hash, "hash, input_len {n}");
        assert_eq!(
            lex_keyed_hash(OFFICIAL_KEY, &input, 131),
            Ok(case.keyed_hash.clone()),
            "keyed_hash, input_len {n}"
        );
        assert_eq!(
            lex_derive_key(OFFICIAL_CONTEXT.as_bytes(), &input, 131),
            case.derive_key,
            "derive_key, input_len {n}"
        );
        assert_eq!(
            lex_hash_hex(&input),
            to_hex(&case.hash[..32]),
            "hashHex, input_len {n}"
        );
        assert_eq!(
            lex_pieces(Mode::Derive(OFFICIAL_CONTEXT.as_bytes()), &[&input], 131),
            Ok(case.derive_key.clone()),
            "hashPieces derive_key, input_len {n}"
        );
        checked += 5;
    }
    eprintln!(
        "B3-01: {checked} official outputs reproduced over {} cases",
        cases.len()
    );
}

/// The gate can fail: one flipped bit in an expected output is reported.
#[test]
fn a_wrong_official_digest_is_rejected_b3_01() {
    let cases = official_cases(&root().join("oracles/blake3-1.8.7/test_vectors.json"));
    let mut wrong = cases[0].hash.clone();
    wrong[0] ^= 1;
    assert_ne!(lex_hash(&official_input(cases[0].input_len), 131), wrong);
}
