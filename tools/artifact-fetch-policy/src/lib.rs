#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Deterministic policy primitives for a future artifact-fetch transport.
//!
//! This crate performs no DNS lookup, socket operation, HTTP request, redirect
//! following, file write, or digest calculation. The caller must use these
//! checks to authorize every hop and must bind the actual connection to one of
//! the validated addresses while preserving TLS hostname verification/SNI.
//! A transport that performs a fresh, unchecked DNS lookup after validation
//! defeats this policy.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// An HTTPS origin represented in canonical form.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HttpsOrigin {
    host: String,
    port: u16,
}

impl HttpsOrigin {
    /// Build an HTTPS origin from a hostname or bare IP literal and nonzero port.
    ///
    /// Hostnames are ASCII DNS names (internationalized names must arrive as
    /// canonical IDNA A-labels). IPv6 literals are supplied without brackets.
    pub fn new(host: &str, port: u16) -> Result<Self, PolicyError> {
        if port == 0 {
            return Err(PolicyError::InvalidPort);
        }

        let host = normalize_host(host)?;
        Ok(Self { host, port })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Errors returned by deterministic artifact-fetch policy checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    InvalidHost,
    InvalidPort,
    EmptyOriginAllowlist,
    ZeroByteLimit,
    OriginNotAllowed(HttpsOrigin),
    EmptyDnsAnswer,
    NonPublicDestination(IpAddr),
    ConnectedPeerNotValidated(IpAddr),
    RedirectLimitExceeded { limit: u8 },
    DeclaredLengthExceeded { declared: u64, limit: u64 },
    DownloadLimitExceeded { received: u64, limit: u64 },
}

/// Immutable limits and administrator-approved HTTPS origins.
#[derive(Clone, Debug)]
pub struct FetchPolicy {
    allowed_origins: BTreeSet<HttpsOrigin>,
    max_redirects: u8,
    max_download_bytes: u64,
}

/// Safe redirect decision. Credentials must come from a separate broker, not a
/// URL. They may only be retained for the exact same normalized origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedirectDecision {
    pub target: HttpsOrigin,
    pub same_origin_credentials_allowed: bool,
}

impl FetchPolicy {
    pub fn new(
        allowed_origins: impl IntoIterator<Item = HttpsOrigin>,
        max_redirects: u8,
        max_download_bytes: u64,
    ) -> Result<Self, PolicyError> {
        let allowed_origins: BTreeSet<_> = allowed_origins.into_iter().collect();
        if allowed_origins.is_empty() {
            return Err(PolicyError::EmptyOriginAllowlist);
        }
        if max_download_bytes == 0 {
            return Err(PolicyError::ZeroByteLimit);
        }

        Ok(Self {
            allowed_origins,
            max_redirects,
            max_download_bytes,
        })
    }

    pub fn authorize_origin(&self, origin: &HttpsOrigin) -> Result<(), PolicyError> {
        if self.allowed_origins.contains(origin) {
            Ok(())
        } else {
            Err(PolicyError::OriginNotAllowed(origin.clone()))
        }
    }

    /// Validate every DNS answer for an allowlisted origin.
    ///
    /// The entire answer set fails closed if even one address is not a
    /// conservative globally routable unicast address. This method does not
    /// resolve DNS; the transport must connect directly to a returned address.
    pub fn authorize_resolution(
        &self,
        origin: &HttpsOrigin,
        answers: &[IpAddr],
    ) -> Result<Vec<IpAddr>, PolicyError> {
        self.authorize_origin(origin)?;
        if answers.is_empty() {
            return Err(PolicyError::EmptyDnsAnswer);
        }

        let mut validated = BTreeSet::new();
        for address in answers {
            if !is_conservative_public_unicast(*address) {
                return Err(PolicyError::NonPublicDestination(*address));
            }
            validated.insert(*address);
        }

        Ok(validated.into_iter().collect())
    }

    /// Confirm that the socket's actual peer is in the exact validated set.
    ///
    /// Call this on every connection attempt and retry. Do not let the HTTP
    /// client perform a second uncontrolled hostname resolution.
    pub fn verify_connected_peer(
        &self,
        origin: &HttpsOrigin,
        validated_addresses: &[IpAddr],
        connected_peer: IpAddr,
    ) -> Result<(), PolicyError> {
        self.authorize_origin(origin)?;
        if !is_conservative_public_unicast(connected_peer) {
            return Err(PolicyError::NonPublicDestination(connected_peer));
        }
        if validated_addresses.contains(&connected_peer) {
            Ok(())
        } else {
            Err(PolicyError::ConnectedPeerNotValidated(connected_peer))
        }
    }

    /// Authorize one redirect target, bounding hop count and origin changes.
    ///
    /// redirects_already_followed is zero for the first response redirect.
    /// The caller must invoke this for every Location hop before opening the
    /// next connection and must independently validate that hop's DNS answers.
    pub fn authorize_redirect(
        &self,
        from: &HttpsOrigin,
        to: &HttpsOrigin,
        redirects_already_followed: u8,
    ) -> Result<RedirectDecision, PolicyError> {
        self.authorize_origin(from)?;
        if redirects_already_followed >= self.max_redirects {
            return Err(PolicyError::RedirectLimitExceeded {
                limit: self.max_redirects,
            });
        }
        self.authorize_origin(to)?;

        Ok(RedirectDecision {
            target: to.clone(),
            same_origin_credentials_allowed: from == to,
        })
    }

    /// Reject an advertised Content-Length above the configured byte cap.
    pub fn check_declared_length(&self, declared: Option<u64>) -> Result<(), PolicyError> {
        if let Some(declared) = declared {
            if declared > self.max_download_bytes {
                return Err(PolicyError::DeclaredLengthExceeded {
                    declared,
                    limit: self.max_download_bytes,
                });
            }
        }
        Ok(())
    }

    /// Reject a streamed byte count above the cap.
    ///
    /// The transport must call this while streaming and abort immediately when
    /// it returns an error; checking only after writing the whole body is too late.
    pub fn check_received_bytes(&self, received: u64) -> Result<(), PolicyError> {
        if received > self.max_download_bytes {
            Err(PolicyError::DownloadLimitExceeded {
                received,
                limit: self.max_download_bytes,
            })
        } else {
            Ok(())
        }
    }

    pub fn max_download_bytes(&self) -> u64 {
        self.max_download_bytes
    }
}

fn normalize_host(host: &str) -> Result<String, PolicyError> {
    if host.is_empty()
        || !host.is_ascii()
        || host.chars().any(|c| {
            c.is_ascii_whitespace()
                || matches!(c, '/' | '\\' | '@' | '?' | '#' | '%' | '[' | ']')
        })
    {
        return Err(PolicyError::InvalidHost);
    }

    // Accept IP literals as host fields, but store their canonical rendering.
    if let Ok(address) = host.parse::<IpAddr>() {
        return Ok(address.to_string());
    }

    // A hostname field is not an authority field: ports and malformed IPv6 are
    // rejected instead of being ambiguously interpreted as host text.
    if host.contains(':') || host.ends_with("..") {
        return Err(PolicyError::InvalidHost);
    }

    let without_root_dot = host.strip_suffix('.').unwrap_or(host);
    if without_root_dot.is_empty() || without_root_dot.len() > 253 {
        return Err(PolicyError::InvalidHost);
    }

    let labels: Vec<&str> = without_root_dot.split('.').collect();
    if labels.iter().any(|label| {
        label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }) {
        return Err(PolicyError::InvalidHost);
    }

    // Prevent invalid numeric-looking dotted strings from being treated as
    // DNS names when a parser or resolver might interpret them as IP literals.
    if labels.iter().all(|label| label.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(PolicyError::InvalidHost);
    }

    Ok(without_root_dot.to_ascii_lowercase())
}

fn is_conservative_public_unicast(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => is_conservative_public_ipv4(ip),
        IpAddr::V6(ip) => is_conservative_public_ipv6(ip),
    }
}

fn is_conservative_public_ipv4(ip: Ipv4Addr) -> bool {
    // Conservative deny list based on IANA's IPv4 Special-Purpose Address
    // Registry. Some entries include globally reachable exceptions; this
    // policy intentionally blocks the whole registered prefix.
    const BLOCKED: [([u8; 4], u8); 18] = [
        ([0, 0, 0, 0], 8),       // this network / unspecified
        ([10, 0, 0, 0], 8),      // RFC 1918 private use
        ([100, 64, 0, 0], 10),   // RFC 6598 shared address space
        ([127, 0, 0, 0], 8),     // loopback
        ([169, 254, 0, 0], 16),  // link-local / cloud metadata ranges
        ([172, 16, 0, 0], 12),   // RFC 1918 private use
        ([192, 0, 0, 0], 24),    // IETF protocol assignments
        ([192, 0, 2, 0], 24),    // documentation
        ([192, 31, 196, 0], 24), // special-purpose AS112
        ([192, 52, 193, 0], 24), // special-purpose AMT
        ([192, 88, 99, 0], 24),  // deprecated 6to4 relay anycast
        ([192, 168, 0, 0], 16),  // RFC 1918 private use
        ([192, 175, 48, 0], 24), // special-purpose AS112
        ([198, 18, 0, 0], 15),   // benchmarking
        ([198, 51, 100, 0], 24), // documentation
        ([203, 0, 113, 0], 24),  // documentation
        ([224, 0, 0, 0], 4),     // multicast
        ([240, 0, 0, 0], 4),     // reserved / limited broadcast
    ];

    !BLOCKED
        .iter()
        .any(|(network, prefix)| ipv4_in_prefix(ip, Ipv4Addr::from(*network), *prefix))
}

fn is_conservative_public_ipv6(ip: Ipv6Addr) -> bool {
    // Only consider global-unicast space, then exclude registered special
    // ranges inside it. This deliberately rejects transition, documentation,
    // multicast, unique-local, link-local, and other non-global addresses.
    let in_global_unicast = ipv6_in_prefix(
        ip,
        Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0),
        3,
    );

    in_global_unicast
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0), 32)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0), 20)
}

fn ipv4_in_prefix(address: Ipv4Addr, network: Ipv4Addr, prefix: u8) -> bool {
    let mask = u32::MAX << (32 - prefix);
    (u32::from(address) & mask) == (u32::from(network) & mask)
}

fn ipv6_in_prefix(address: Ipv6Addr, network: Ipv6Addr, prefix: u8) -> bool {
    let mask = u128::MAX << (128 - prefix);
    (u128::from(address) & mask) == (u128::from(network) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(host: &str, port: u16) -> HttpsOrigin {
        HttpsOrigin::new(host, port).expect("fixture origin should be valid")
    }

    fn policy() -> FetchPolicy {
        FetchPolicy::new(
            [
                origin("REGISTRY.EXAMPLE.INVALID.", 443),
                origin("mirror.example.invalid", 8443),
            ],
            3,
            1024,
        )
        .expect("fixture policy should be valid")
    }

    fn ip(value: &str) -> IpAddr {
        value.parse().expect("fixture IP address should parse")
    }

    #[test]
    fn canonicalizes_dns_names_and_ip_literals() {
        assert_eq!(
            origin("REGISTRY.EXAMPLE.INVALID.", 443).host(),
            "registry.example.invalid"
        );
        assert_eq!(origin("2001:4860:4860::8888", 443).host(), "2001:4860:4860::8888");
        assert_eq!(origin("8.8.8.8", 443).host(), "8.8.8.8");
    }

    #[test]
    fn rejects_ambiguous_host_fields_and_zero_port() {
        for host in [
            "",
            " bad.example",
            "bad.example/path",
            "user@bad.example",
            "bad.example?query",
            "bad.example#fragment",
            "bad%2eexample",
            r"bad\example",
            "bad..example",
            "bad..",
            "-bad.example",
            "bad-.example",
            "999.999.999.999",
            "0177.0.0.1",
            "example:443",
            "2001:db8::zz",
            "bad_label.example",
        ] {
            assert_eq!(HttpsOrigin::new(host, 443), Err(PolicyError::InvalidHost), "{host}");
        }
        assert_eq!(HttpsOrigin::new("example.invalid", 0), Err(PolicyError::InvalidPort));
    }

    #[test]
    fn requires_nonempty_allowlist_and_positive_size_limit() {
        assert!(matches!(
            FetchPolicy::new([], 0, 1),
            Err(PolicyError::EmptyOriginAllowlist)
        ));
        assert!(matches!(
            FetchPolicy::new([origin("example.invalid", 443)], 0, 0),
            Err(PolicyError::ZeroByteLimit)
        ));
    }

    #[test]
    fn origin_allowlist_is_exact_after_normalization() {
        let p = policy();
        assert!(p.authorize_origin(&origin("registry.example.invalid", 443)).is_ok());
        assert!(p.authorize_origin(&origin("mirror.example.invalid", 8443)).is_ok());
        assert!(matches!(
            p.authorize_origin(&origin("attacker.example.invalid", 443)),
            Err(PolicyError::OriginNotAllowed(_))
        ));
        assert!(matches!(
            p.authorize_origin(&origin("registry.example.invalid", 8443)),
            Err(PolicyError::OriginNotAllowed(_))
        ));
    }

    #[test]
    fn accepts_public_ipv4_and_ipv6_answers() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let addresses = [ip("8.8.8.8"), ip("2606:4700:4700::1111")];
        let validated = p.authorize_resolution(&registry, &addresses).unwrap();
        assert_eq!(validated.len(), 2);
        assert!(validated.contains(&ip("8.8.8.8")));
        assert!(validated.contains(&ip("2606:4700:4700::1111")));
    }

    #[test]
    fn rejects_empty_resolution_and_any_mixed_private_answer() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        assert_eq!(
            p.authorize_resolution(&registry, &[]),
            Err(PolicyError::EmptyDnsAnswer)
        );
        assert_eq!(
            p.authorize_resolution(&registry, &[ip("8.8.8.8"), ip("10.0.0.7")]),
            Err(PolicyError::NonPublicDestination(ip("10.0.0.7")))
        );
        assert_eq!(
            p.authorize_resolution(&registry, &[ip("8.8.8.8"), ip("169.254.169.254")]),
            Err(PolicyError::NonPublicDestination(ip("169.254.169.254")))
        );
    }

    #[test]
    fn rejects_ipv4_special_purpose_ranges() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        for address in [
            "0.0.0.0",
            "10.1.2.3",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.0.0.9",
            "192.0.2.1",
            "192.31.196.1",
            "192.52.193.1",
            "192.88.99.1",
            "192.168.1.1",
            "192.175.48.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
        ] {
            let address = ip(address);
            assert_eq!(
                p.authorize_resolution(&registry, &[address]),
                Err(PolicyError::NonPublicDestination(address)),
                "{address}"
            );
        }
    }

    #[test]
    fn rejects_non_global_ipv6_and_transition_ranges() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        for address in [
            "::",
            "::1",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
            "100::1",
            "2001::1",
            "2001:db8::1",
            "2002::1",
            "3fff::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            let address = ip(address);
            assert_eq!(
                p.authorize_resolution(&registry, &[address]),
                Err(PolicyError::NonPublicDestination(address)),
                "{address}"
            );
        }
    }

    #[test]
    fn actual_socket_peer_must_match_validated_resolution() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let validated = p.authorize_resolution(&registry, &[ip("8.8.8.8")]).unwrap();
        assert!(p.verify_connected_peer(&registry, &validated, ip("8.8.8.8")).is_ok());
        assert_eq!(
            p.verify_connected_peer(&registry, &validated, ip("1.1.1.1")),
            Err(PolicyError::ConnectedPeerNotValidated(ip("1.1.1.1")))
        );
        assert_eq!(
            p.verify_connected_peer(&registry, &validated, ip("127.0.0.1")),
            Err(PolicyError::NonPublicDestination(ip("127.0.0.1")))
        );
    }

    #[test]
    fn redirects_require_allowlisted_origins_and_bounded_hops() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let mirror = origin("mirror.example.invalid", 8443);
        let same = p.authorize_redirect(&registry, &registry, 0).unwrap();
        assert!(same.same_origin_credentials_allowed);

        let cross = p.authorize_redirect(&registry, &mirror, 0).unwrap();
        assert_eq!(cross.target, mirror);
        assert!(!cross.same_origin_credentials_allowed);

        assert_eq!(
            p.authorize_redirect(&registry, &mirror, 3),
            Err(PolicyError::RedirectLimitExceeded { limit: 3 })
        );
        assert!(matches!(
            p.authorize_redirect(&registry, &origin("attacker.example.invalid", 443), 0),
            Err(PolicyError::OriginNotAllowed(_))
        ));
    }

    #[test]
    fn declared_and_streamed_content_lengths_are_bounded() {
        let p = policy();
        assert!(p.check_declared_length(None).is_ok());
        assert!(p.check_declared_length(Some(1024)).is_ok());
        assert_eq!(
            p.check_declared_length(Some(1025)),
            Err(PolicyError::DeclaredLengthExceeded {
                declared: 1025,
                limit: 1024
            })
        );
        assert!(p.check_received_bytes(1024).is_ok());
        assert_eq!(
            p.check_received_bytes(1025),
            Err(PolicyError::DownloadLimitExceeded {
                received: 1025,
                limit: 1024
            })
        );
        assert_eq!(p.max_download_bytes(), 1024);
    }
}
