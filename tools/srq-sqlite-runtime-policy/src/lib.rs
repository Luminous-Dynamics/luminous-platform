//! Shared fail-closed SQLite runtime release policy for SRQ durable stores.
//!
//! This crate intentionally has no dependencies. Keeping the policy in one
//! pure module prevents the D1 receipt store and policy-promotion store from
//! quietly accepting different SQLite release sets.
//!
//! The WAL-reset corruption fix landed in SQLite 3.51.3, with backports to
//! 3.44.6 and 3.50.7. Later, upstream released fixes for defects in 3.53.0
//! through 3.53.3, and 3.54.1 fixed a heap-buffer-overflow in 3.54.0 on some
//! 32-bit builds. SRQ rejects those known-buggy lines across architectures.
//! This is a maintained minimum policy, not a claim that any accepted SQLite
//! release is defect-free. Revisit it when upstream publishes new advisories.

/// Stable identifier to include in diagnostics and engineering evidence.
pub const POLICY_ID: &str = "SRQ-SQLITE-RUNTIME-POLICY-2026-10";

/// Parse an exact SQLite major.minor.patch release string.
///
/// SQLite's sqlite_version() returns a version in this shape. Suffixes,
/// missing components, empty components, and extra components fail closed.
pub fn parse_sqlite_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Return whether a SQLite version satisfies the explicit SRQ release policy.
pub fn is_supported_sqlite_version(value: &str) -> bool {
    parse_sqlite_version(value)
        .is_some_and(is_supported_sqlite_version_tuple)
}

/// Version tuple form used by table-driven tests and callers that already parsed
/// a version string.
pub fn is_supported_sqlite_version_tuple((major, minor, patch): (u64, u64, u64)) -> bool {
    match (major, minor) {
        (3, 44) => patch >= 6,
        (3, 50) => patch >= 7,
        (3, 51) => patch >= 3,
        (3, 52) => false, // withdrawn release line
        (3, 53) => patch >= 4, // reject known defects in 3.53.0-3.53.3
        (3, 54) => patch >= 1, // reject 3.54.0 across architectures
        // New minor/major lines require an explicit upstream review and policy change.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_upstream_fixed_backports_and_patched_release_lines() {
        for version in [
            "3.44.6",
            "3.50.7",
            "3.51.3",
            "3.53.4",
            "3.53.99",
            "3.54.1",
            "3.54.9",
        ] {
            assert!(
                is_supported_sqlite_version(version),
                "expected supported SQLite version: {version}"
            );
        }
    }

    #[test]
    fn rejects_known_buggy_withdrawn_and_unpatched_release_lines() {
        for version in [
            "3.44.5",
            "3.50.6",
            "3.51.2",
            "3.52.0",
            "3.52.1",
            "3.53.0",
            "3.53.1",
            "3.53.2",
            "3.53.3",
            "3.54.0",
            "3.55.0",
            "3.56.0",
            "4.0.0",
        ] {
            assert!(
                !is_supported_sqlite_version(version),
                "expected rejected SQLite version: {version}"
            );
        }
    }

    #[test]
    fn malformed_or_incomplete_versions_fail_closed() {
        for version in ["", "3", "3.51", "3..3", "3.51.3-extra", "3.54.1.2", "v3.54.1"] {
            assert!(
                !is_supported_sqlite_version(version),
                "malformed SQLite version must be rejected: {version}"
            );
        }
        assert_eq!(parse_sqlite_version("3.54.1"), Some((3, 54, 1)));
        assert_eq!(parse_sqlite_version("3.54.1-extra"), None);
    }
}
