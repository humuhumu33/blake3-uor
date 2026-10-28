//! `B3-05`: the committed packages are the ones a verified LexLean build of
//! the model exported, and that build checked every kernel theorem.
//!
//! Reproducing the export needs LexLean and Lean, which the SDK image does not
//! carry; this test checks what can be checked from the committed bytes: each
//! package file hashes to its provenance record, each package names the
//! semantic identity the attestation verified, and the attestation holds every
//! theorem of `lexlean/src/` with its exact axiom policy.

use std::collections::BTreeSet;
use std::path::Path;

use blake3_check::root;
use serde_json::Value;

const PACKAGES: &[&str] = &[
    "blake3_derive_key",
    "blake3_hash",
    "blake3_hash_hex",
    "blake3_hash_pieces",
    "blake3_keyed_hash",
];

fn json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The theorem names in the committed theorem modules, read from source.
fn theorems_in_source(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut entries: Vec<_> = std::fs::read_dir(root.join("lexlean/src"))
        .expect("lexlean/src")
        .map(|e| e.expect("entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let text = std::fs::read_to_string(&path).expect("module");
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            if line.trim() == "theorem{" {
                let name = lines.next().expect("a name follows").trim();
                let name = name
                    .strip_prefix("name: ")
                    .and_then(|n| n.strip_suffix(','))
                    .expect("theorem name line");
                names.insert(name.to_string());
            }
        }
    }
    names
}

#[test]
fn the_export_is_what_a_verified_build_wrote_b3_05() {
    let root = root();
    let export = root.join("export");
    let attestation = json(&export.join("attestation.json"));
    assert_eq!(attestation["status"], "verified");
    let semantic = attestation["semantic_id"].as_str().expect("semantic_id");

    for name in PACKAGES {
        let dir = export.join("rust-std").join(name);
        let provenance = json(&dir.join("provenance.json"));
        assert_eq!(provenance["name"], *name);
        assert_eq!(provenance["profile"], "rust-std");
        assert_eq!(
            provenance["sources"],
            serde_json::json!([semantic]),
            "{name} was exported from the attested model"
        );
        let files = provenance["files"].as_object().expect("files");
        assert_eq!(files.len(), 2, "{name}: exactly Cargo.toml and src/lib.rs");
        for (file, digest) in files {
            let bytes = std::fs::read(dir.join(file)).expect("package file");
            assert_eq!(
                sha256(&bytes),
                digest.as_str().expect("hex"),
                "{name}/{file}"
            );
        }
    }

    let theorems = theorems_in_source(&root);
    assert_eq!(theorems.len(), 11, "eleven kernel theorems");
    let declarations = attestation["declarations"]
        .as_array()
        .expect("declarations");
    for theorem in &theorems {
        let row = declarations
            .iter()
            .find(|d| {
                d["name"]
                    .as_str()
                    .is_some_and(|n| n.ends_with(&format!(".{theorem}")))
            })
            .unwrap_or_else(|| panic!("{theorem} is not in the attestation"));
        assert_eq!(row["result"], "ok", "{theorem}");
        assert_eq!(
            row["observed"],
            serde_json::json!(["Quot.sound", "propext"]),
            "{theorem}: exactly the declared axioms"
        );
    }
    eprintln!(
        "B3-05: {} packages bound to attested model {semantic}, {} theorems verified",
        PACKAGES.len(),
        theorems.len()
    );
}

/// The binding can fail: one changed byte of a package no longer hashes to
/// its provenance record.
#[test]
fn a_changed_package_byte_is_detected_b3_05() {
    let dir = root().join("export/rust-std/blake3_hash");
    let provenance = json(&dir.join("provenance.json"));
    let mut bytes = std::fs::read(dir.join("src/lib.rs")).expect("lib.rs");
    assert_eq!(sha256(&bytes), provenance["files"]["src/lib.rs"]);
    bytes[0] ^= 1;
    assert_ne!(sha256(&bytes), provenance["files"]["src/lib.rs"]);
}
