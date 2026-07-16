//! Proves the two dialect-hash pipelines agree (Phase 0.3 `hash.sh` and
//! this crate's `build.rs`), closing the loop the golden test opens:
//!
//! * `hash.sh` (shell, shasum/sha256sum) writes the committed
//!   `cc-dialect/generated/dialect_hash.rs`; the same value goes into the
//!   C header the PX4 side and the golden generator compile against.
//! * `build.rs` (Rust, sha2 crate) computes `cc_protocol::dialect_hash::*`
//!   from the same XML at every build.
//!
//! If they disagree, either the XML changed without re-running the scripts
//! (stale committed artifacts — regenerate) or one of the two hash
//! implementations broke.

// The committed artifact, included verbatim as source:
mod committed {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../cc-dialect/generated/dialect_hash.rs"
    ));
}

#[test]
fn committed_hash_matches_build_computed_hash() {
    assert_eq!(
        committed::CC_DIALECT_HASH,
        cc_protocol::dialect_hash::CC_DIALECT_HASH,
        "hash.sh output is stale or a hash pipeline broke — run cc-dialect/hash.sh"
    );
    assert_eq!(
        committed::CC_DIALECT_SHA256,
        cc_protocol::dialect_hash::CC_DIALECT_SHA256,
        "hash.sh output is stale or a hash pipeline broke — run cc-dialect/hash.sh"
    );
}

#[test]
fn hash_matches_truncation_rule() {
    // CC_DIALECT_HASH is defined as the first 8 hex chars of the SHA-256,
    // read as a big-endian u32 — pin the rule itself.
    let head = &cc_protocol::dialect_hash::CC_DIALECT_SHA256[..8];
    let from_hex = u32::from_str_radix(head, 16).expect("sha256 must be hex");
    assert_eq!(from_hex, cc_protocol::dialect_hash::CC_DIALECT_HASH);
}
