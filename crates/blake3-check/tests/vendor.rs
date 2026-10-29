//! `B3-12`: the vendored LexLean stdlib is upstream's, byte for byte.
//!
//! LexLean reads a source package only from a directory of the workspace
//! (SPEC 10.5), so the stdlib that supplies the sha256 axis is a copy. This
//! gate holds the copy to the cited upstream commit.

use std::path::Path;

use blake3_check::{root, to_hex};
use repo_model::Model;
use sha2::Digest as _;

const FILES: [&str; 3] = ["lexlean.toml", "src/Sha256.lex.tex", "src/Text.lex.tex"];

fn sha256(bytes: &[u8]) -> String {
    to_hex(&sha2::Sha256::digest(bytes))
}

/// `<path> <sha256>` per file, LF-terminated, in path order.
fn listing(read: &dyn Fn(&str) -> Vec<u8>) -> String {
    FILES
        .iter()
        .map(|f| format!("{f} {}\n", sha256(&read(f))))
        .collect()
}

fn files_under(dir: &Path, base: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("readable") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            files_under(&path, base, out);
        } else {
            let relative = path.strip_prefix(base).expect("inside");
            let parts: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
}

#[test]
fn the_vendored_stdlib_is_upstream_byte_for_byte_b3_12() {
    let root = root();
    let vendored = root.join("vendor/lexlean-stdlib");
    let mut present = Vec::new();
    files_under(&vendored, &vendored, &mut present);
    present.sort();
    assert_eq!(present, FILES, "exactly the upstream files, nothing added");

    let model = Model::load(&root.join("model")).expect("the model loads");
    let cited = model
        .authorities
        .authority
        .iter()
        .find(|a| a.id == "AUTH-LEXLEAN-STDLIB")
        .expect("the stdlib is a cited authority");
    let read = |f: &str| std::fs::read(vendored.join(f)).expect("a vendored file");
    let digest = format!("sha256:{}", sha256(listing(&read).as_bytes()));
    assert_eq!(
        digest, cited.checksum,
        "vendor/lexlean-stdlib differs from LexLean 6bcc8d598"
    );

    let lock =
        std::fs::read_to_string(root.join("lexlean-trace/lexlean.lock")).expect("the trace lock");
    let manifest = format!("manifest_sha256 = \"{}\"", sha256(&read("lexlean.toml")));
    assert!(
        lock.contains("source = \"vendor/lexlean-stdlib\"") && lock.contains(&manifest),
        "the trace project's lock pins the vendored manifest"
    );
}

/// The gate can fail: one changed byte of the copy changes the listing.
#[test]
fn a_changed_vendored_byte_is_detected_b3_12() {
    let vendored = root().join("vendor/lexlean-stdlib");
    let read = |f: &str| std::fs::read(vendored.join(f)).expect("a vendored file");
    let changed = |f: &str| {
        let mut bytes = read(f);
        if f == "src/Sha256.lex.tex" {
            bytes[100] ^= 1;
        }
        bytes
    };
    assert_ne!(
        sha256(listing(&read).as_bytes()),
        sha256(listing(&changed).as_bytes())
    );
}
