//! `B3-05`: the committed packages are the ones a verified LexLean build of
//! the model exported, and that build checked every kernel theorem.
//!
//! Reproducing the export needs LexLean and Lean, which the SDK image does not
//! carry; this test checks what can be checked from the committed bytes: each
//! package file hashes to its provenance record, each package names the
//! semantic identity the attestation verified, and each project's
//! attestation holds every theorem of its `src/` with its exact axiom policy.

use std::collections::BTreeSet;
use std::path::Path;

use blake3_check::root;
use serde_json::Value;

/// Each exported package and the attestation of the project that exported it.
const PACKAGES: &[(&str, &str)] = &[
    ("blake3_derive_key", "attestation.json"),
    ("blake3_hash", "attestation.json"),
    ("blake3_hash_hex", "attestation.json"),
    ("blake3_hash_pieces", "attestation.json"),
    ("blake3_keyed_hash", "attestation.json"),
    ("blake3_trace_address", "attestation-trace.json"),
    ("blake3_trace_file", "attestation-trace.json"),
    ("blake3_slice_check", "attestation-slice.json"),
    ("blake3_slice_of", "attestation-slice.json"),
];

/// Each project with kernel theorems, its attestation, and how many.
const THEOREM_PROJECTS: &[(&str, &str, usize)] = &[
    ("lexlean", "attestation.json", 16),
    ("lexlean-reverse", "attestation-reverse.json", 1),
    ("lexlean-slice", "attestation-slice.json", 3),
    ("lexlean-analysis", "attestation-analysis.json", 1),
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

/// The theorem names in a project's committed modules, read from source.
fn theorems_in_source(project: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut entries: Vec<_> = std::fs::read_dir(project.join("src"))
        .expect("src")
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
    for (name, attested) in PACKAGES {
        let attestation = json(&export.join(attested));
        assert_eq!(attestation["status"], "verified", "{attested}");
        let semantic = attestation["semantic_id"].as_str().expect("semantic_id");
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

    let mut verified = 0;
    for (project, attested, count) in THEOREM_PROJECTS {
        let attestation = json(&export.join(attested));
        assert_eq!(attestation["status"], "verified", "{attested}");
        let theorems = theorems_in_source(&root.join(project));
        assert_eq!(theorems.len(), *count, "{project}: its kernel theorems");
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
                .unwrap_or_else(|| panic!("{theorem} is not in {attested}"));
            assert_eq!(row["result"], "ok", "{theorem}");
            assert_eq!(
                row["observed"],
                serde_json::json!(["Quot.sound", "propext"]),
                "{theorem}: exactly the declared axioms"
            );
        }
        verified += theorems.len();
    }
    eprintln!(
        "B3-05: {} packages bound to their attested models, {verified} theorems verified",
        PACKAGES.len()
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
