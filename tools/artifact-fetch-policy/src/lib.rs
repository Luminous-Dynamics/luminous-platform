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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

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

/// Validate an HTTP `Content-Encoding` value for a byte-bounded artifact fetch.
///
/// Only an absent header or `identity` is accepted. Compressed representations
/// must be explicitly supported by a transport that meters both wire bytes and
/// decoded artifact bytes; this policy intentionally fails closed by default.
pub fn validate_content_encoding_headers(encodings: &[&str]) -> Result<(), PolicyError> {
    match encodings {
        [] => Ok(()),
        [value] if value.trim().eq_ignore_ascii_case("identity") => Ok(()),
        _ => Err(PolicyError::UnsupportedContentEncoding),
    }
}

/// Errors returned by deterministic artifact-fetch policy checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    InvalidHost,
    InvalidPort,
    EmptyOriginAllowlist,
    ZeroByteLimit,
    ZeroTimeout,
    TimeoutExceedsTotal {
        phase: &'static str,
    },
    OriginNotAllowed(HttpsOrigin),
    EmptyDnsAnswer,
    IpLiteralResolutionMismatch {
        expected: IpAddr,
    },
    NonPublicDestination(IpAddr),
    ResolutionOriginMismatch {
        requested: HttpsOrigin,
        validated_for: HttpsOrigin,
    },
    ConnectedPeerNotValidated(IpAddr),
    ConnectedPeerPortMismatch {
        expected: u16,
        actual: u16,
    },
    RedirectLimitExceeded {
        limit: u8,
    },
    DeclaredLengthExceeded {
        declared: u64,
        limit: u64,
    },
    DownloadLimitExceeded {
        received: u64,
        limit: u64,
    },
    DownloadCounterOverflow,
    UnsupportedContentEncoding,
    InvalidContentLength,
    AmbiguousContentLength,
    RedirectChainFailed,
}

/// Cumulative byte meter for one artifact transfer.
///
/// Once any chunk would exceed the configured limit, this meter becomes
/// terminal: later calls remain rejected. Construct it through `FetchPolicy`
/// so the limit comes from the administrator's immutable policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DownloadFailure {
    LimitExceeded,
    CounterOverflow,
}

#[must_use = "download budget must be retained for the entire transfer"]
#[derive(Debug, Eq, PartialEq)]
pub struct DownloadBudget {
    limit: u64,
    received: u64,
    failure: Option<DownloadFailure>,
}

impl DownloadBudget {
    /// Account for the next body chunk before accepting or writing that chunk.
    /// Returns the cumulative byte count only while it remains within policy.
    pub fn record_chunk(&mut self, chunk_bytes: usize) -> Result<u64, PolicyError> {
        if let Some(failure) = self.failure {
            return Err(match failure {
                DownloadFailure::LimitExceeded => PolicyError::DownloadLimitExceeded {
                    received: self.received,
                    limit: self.limit,
                },
                DownloadFailure::CounterOverflow => PolicyError::DownloadCounterOverflow,
            });
        }

        let Ok(chunk_bytes) = u64::try_from(chunk_bytes) else {
            self.received = u64::MAX;
            self.failure = Some(DownloadFailure::CounterOverflow);
            return Err(PolicyError::DownloadCounterOverflow);
        };
        let Some(received) = self.received.checked_add(chunk_bytes) else {
            self.received = u64::MAX;
            self.failure = Some(DownloadFailure::CounterOverflow);
            return Err(PolicyError::DownloadCounterOverflow);
        };

        self.received = received;
        if received > self.limit {
            self.failure = Some(DownloadFailure::LimitExceeded);
            Err(PolicyError::DownloadLimitExceeded {
                received,
                limit: self.limit,
            })
        } else {
            Ok(received)
        }
    }

    pub fn received_bytes(&self) -> u64 {
        self.received
    }

    pub fn is_failed(&self) -> bool {
        self.failure.is_some()
    }
}

/// DNS answers that passed policy checks for one exact HTTPS origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedResolution {
    origin: HttpsOrigin,
    addresses: Vec<IpAddr>,
}

impl ValidatedResolution {
    pub fn origin(&self) -> &HttpsOrigin {
        &self.origin
    }

    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }
}

/// Time limits that a concrete transport must enforce.
///
/// `total` is one deadline for the entire fetch, including redirects and
/// retries; transports must not restart this budget on each attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchTimeouts {
    connect: Duration,
    read_idle: Duration,
    total: Duration,
}

impl FetchTimeouts {
    /// Build positive connect, read-idle, and whole-fetch time limits.
    pub fn new(
        connect: Duration,
        read_idle: Duration,
        total: Duration,
    ) -> Result<Self, PolicyError> {
        if connect.is_zero() || read_idle.is_zero() || total.is_zero() {
            return Err(PolicyError::ZeroTimeout);
        }
        if connect > total {
            return Err(PolicyError::TimeoutExceedsTotal { phase: "connect" });
        }
        if read_idle > total {
            return Err(PolicyError::TimeoutExceedsTotal { phase: "read_idle" });
        }

        Ok(Self {
            connect,
            read_idle,
            total,
        })
    }

    pub fn connect(&self) -> Duration {
        self.connect
    }

    pub fn read_idle(&self) -> Duration {
        self.read_idle
    }

    pub fn total(&self) -> Duration {
        self.total
    }
}

/// Immutable limits and administrator-approved HTTPS origins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchPolicy {
    allowed_origins: BTreeSet<HttpsOrigin>,
    max_redirects: u8,
    max_download_bytes: u64,
    timeouts: FetchTimeouts,
}

/// Redirect-chain state. Private fields and no `Clone` implementation prevent
/// callers from resetting or trivially forking the per-chain hop counter.
#[must_use = "redirect chain must be retained across every redirect hop"]
#[derive(Debug, Eq, PartialEq)]
pub struct RedirectChain {
    current: HttpsOrigin,
    allowed_origins: BTreeSet<HttpsOrigin>,
    redirects_followed: u8,
    max_redirects: u8,
    failed: bool,
}

impl RedirectChain {
    pub fn current_origin(&self) -> &HttpsOrigin {
        &self.current
    }

    pub fn redirects_followed(&self) -> u8 {
        self.redirects_followed
    }

    pub fn is_failed(&self) -> bool {
        self.failed
    }
}

/// Safe redirect decision. Fields are private so callers cannot fabricate an
/// authorization result; credentials are still managed by the broker, not URLs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedirectDecision {
    target: HttpsOrigin,
    same_origin_credentials_allowed: bool,
}

impl RedirectDecision {
    pub fn target(&self) -> &HttpsOrigin {
        &self.target
    }

    pub fn same_origin_credentials_allowed(&self) -> bool {
        self.same_origin_credentials_allowed
    }
}

impl FetchPolicy {
    pub fn new(
        allowed_origins: impl IntoIterator<Item = HttpsOrigin>,
        max_redirects: u8,
        max_download_bytes: u64,
        timeouts: FetchTimeouts,
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
            timeouts,
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
    ) -> Result<ValidatedResolution, PolicyError> {
        self.authorize_origin(origin)?;
        if answers.is_empty() {
            return Err(PolicyError::EmptyDnsAnswer);
        }

        // IP-literal URLs have no DNS rebinding step: the selected peer must
        // be that literal, not an arbitrary public address supplied as an
        // alleged resolution result.
        if let Ok(expected) = origin.host.parse::<IpAddr>() {
            if answers.len() != 1 || answers[0] != expected {
                return Err(PolicyError::IpLiteralResolutionMismatch { expected });
            }
        }

        let mut validated = BTreeSet::new();
        for address in answers {
            if !is_conservative_public_unicast(*address) {
                return Err(PolicyError::NonPublicDestination(*address));
            }
            validated.insert(*address);
        }

        Ok(ValidatedResolution {
            origin: origin.clone(),
            addresses: validated.into_iter().collect(),
        })
    }

    /// Confirm that the socket's actual peer is in the exact validated set.
    ///
    /// Call this on every connection attempt and retry. Do not let the HTTP
    /// client perform a second uncontrolled hostname resolution.
    pub fn verify_connected_peer(
        &self,
        origin: &HttpsOrigin,
        validated: &ValidatedResolution,
        connected_peer: SocketAddr,
    ) -> Result<(), PolicyError> {
        if &validated.origin != origin {
            return Err(PolicyError::ResolutionOriginMismatch {
                requested: origin.clone(),
                validated_for: validated.origin.clone(),
            });
        }
        self.authorize_origin(origin)?;
        let peer_ip = connected_peer.ip();
        if !is_conservative_public_unicast(peer_ip) {
            return Err(PolicyError::NonPublicDestination(peer_ip));
        }
        if connected_peer.port() != origin.port() {
            return Err(PolicyError::ConnectedPeerPortMismatch {
                expected: origin.port(),
                actual: connected_peer.port(),
            });
        }
        if validated.addresses.contains(&peer_ip) {
            Ok(())
        } else {
            Err(PolicyError::ConnectedPeerNotValidated(peer_ip))
        }
    }

    /// Start a redirect chain at an authorized initial origin.
    pub fn start_redirect_chain(
        &self,
        initial_origin: &HttpsOrigin,
    ) -> Result<RedirectChain, PolicyError> {
        self.authorize_origin(initial_origin)?;
        Ok(RedirectChain {
            current: initial_origin.clone(),
            allowed_origins: self.allowed_origins.clone(),
            redirects_followed: 0,
            max_redirects: self.max_redirects,
            failed: false,
        })
    }

    /// Authorize and account for exactly one redirect from the chain's current
    /// origin. The caller cannot supply or reset the hop count. Any rejected
    /// hop makes the chain terminal; start a new fetch rather than continuing
    /// an HTTP redirect chain after denial.
    pub fn authorize_redirect_hop(
        &self,
        chain: &mut RedirectChain,
        to: &HttpsOrigin,
    ) -> Result<RedirectDecision, PolicyError> {
        if chain.failed {
            return Err(PolicyError::RedirectChainFailed);
        }
        if !chain.allowed_origins.contains(&chain.current) {
            chain.failed = true;
            return Err(PolicyError::OriginNotAllowed(chain.current.clone()));
        }
        if let Err(error) = self.authorize_origin(&chain.current) {
            chain.failed = true;
            return Err(error);
        }

        // Bind each chain to the originating policy's allowlist and preserve
        // the strictest hop limit if another policy instance is passed by error.
        let limit = self.max_redirects.min(chain.max_redirects);
        if chain.redirects_followed >= limit {
            chain.failed = true;
            return Err(PolicyError::RedirectLimitExceeded { limit });
        }
        if !chain.allowed_origins.contains(to) {
            chain.failed = true;
            return Err(PolicyError::OriginNotAllowed(to.clone()));
        }
        if let Err(error) = self.authorize_origin(to) {
            chain.failed = true;
            return Err(error);
        }

        let decision = RedirectDecision {
            target: to.clone(),
            same_origin_credentials_allowed: &chain.current == to,
        };
        chain.current = to.clone();
        chain.redirects_followed += 1;
        Ok(decision)
    }

    /// Validate all raw Content-Length field values from one HTTP response.
    ///
    /// The strict artifact-fetch profile accepts either no field or exactly
    /// one decimal value. It rejects duplicates even when identical, as well as
    /// comma-joined values, non-decimal syntax, and integer overflow. This avoids
    /// relying on differing HTTP stacks' interpretations of ambiguous framing.
    pub fn validate_content_length_headers(
        &self,
        values: &[&str],
    ) -> Result<Option<u64>, PolicyError> {
        let value = match values {
            [] => return Ok(None),
            [value] => *value,
            _ => return Err(PolicyError::AmbiguousContentLength),
        };
        let value = value.trim_matches(|character| character == ' ' || character == '\t');
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(PolicyError::InvalidContentLength);
        }
        let declared = value
            .parse::<u64>()
            .map_err(|_| PolicyError::InvalidContentLength)?;
        self.check_declared_length(Some(declared))?;
        Ok(Some(declared))
    }

    /// Check an already parsed Content-Length number against the configured cap.
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

    /// Create a terminal cumulative byte meter for one transfer.
    pub fn start_download(&self) -> DownloadBudget {
        DownloadBudget {
            limit: self.max_download_bytes,
            received: 0,
            failure: None,
        }
    }

    /// Check an externally maintained streamed-byte total against the cap.
    ///
    /// Prefer `start_download` and `DownloadBudget::record_chunk` in a concrete
    /// transport so overflow and repeated calls after rejection fail closed.
    /// The transport must abort immediately on error; checking only after
    /// writing the whole body is too late.
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

    /// Time budgets the concrete transport is required to enforce.
    pub fn timeouts(&self) -> &FetchTimeouts {
        &self.timeouts
    }
}

fn normalize_host(host: &str) -> Result<String, PolicyError> {
    if host.is_empty()
        || !host.is_ascii()
        || host.chars().any(|c| {
            c.is_ascii_whitespace() || matches!(c, '/' | '\\' | '@' | '?' | '#' | '%' | '[' | ']')
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

    // URL parsers can treat a numeric final label as a shortened, octal,
    // decimal, or hexadecimal IPv4 literal. Reject ambiguous forms rather than
    // letting the URL parser, resolver, and policy interpret them differently.
    if labels
        .last()
        .is_some_and(|label| is_legacy_ipv4_number(label))
    {
        return Err(PolicyError::InvalidHost);
    }

    Ok(without_root_dot.to_ascii_lowercase())
}

fn is_legacy_ipv4_number(label: &str) -> bool {
    if let Some(digits) = label
        .strip_prefix("0x")
        .or_else(|| label.strip_prefix("0X"))
    {
        return digits.bytes().all(|byte| byte.is_ascii_hexdigit());
    }

    !label.is_empty() && label.bytes().all(|byte| byte.is_ascii_digit())
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
    let in_global_unicast = ipv6_in_prefix(ip, Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0), 3);

    in_global_unicast
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0), 32)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0), 20)
        && !ipv6_in_prefix(ip, Ipv6Addr::new(0x5f00, 0, 0, 0, 0, 0, 0, 0), 16)
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

    fn timeouts() -> FetchTimeouts {
        FetchTimeouts::new(
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(60),
        )
        .expect("fixture timeouts should be valid")
    }

    fn policy() -> FetchPolicy {
        FetchPolicy::new(
            [
                origin("REGISTRY.EXAMPLE.INVALID.", 443),
                origin("mirror.example.invalid", 8443),
            ],
            3,
            1024,
            timeouts(),
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
        assert_eq!(
            origin("2001:4860:4860::8888", 443).host(),
            "2001:4860:4860::8888"
        );
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
            "0x7f000001",
            "0x7f.1",
            "127.0.0x1",
            "example.123",
            "example.0x2a",
            "example:443",
            "2001:db8::zz",
            "bad_label.example",
        ] {
            assert_eq!(
                HttpsOrigin::new(host, 443),
                Err(PolicyError::InvalidHost),
                "{host}"
            );
        }
        assert_eq!(
            HttpsOrigin::new("example.invalid", 0),
            Err(PolicyError::InvalidPort)
        );
    }

    #[test]
    fn requires_nonempty_allowlist_and_positive_size_limit() {
        assert!(matches!(
            FetchPolicy::new([], 0, 1, timeouts()),
            Err(PolicyError::EmptyOriginAllowlist)
        ));
        assert!(matches!(
            FetchPolicy::new([origin("example.invalid", 443)], 0, 0, timeouts()),
            Err(PolicyError::ZeroByteLimit)
        ));
    }

    #[test]
    fn timeout_budgets_are_positive_bounded_and_attached_to_policy() {
        assert_eq!(
            FetchTimeouts::new(
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_secs(5)
            ),
            Err(PolicyError::ZeroTimeout)
        );
        assert_eq!(
            FetchTimeouts::new(
                Duration::from_secs(6),
                Duration::from_secs(1),
                Duration::from_secs(5)
            ),
            Err(PolicyError::TimeoutExceedsTotal { phase: "connect" })
        );
        assert_eq!(
            FetchTimeouts::new(
                Duration::from_secs(1),
                Duration::from_secs(6),
                Duration::from_secs(5)
            ),
            Err(PolicyError::TimeoutExceedsTotal { phase: "read_idle" })
        );

        let p = policy();
        assert_eq!(p.timeouts().connect(), Duration::from_secs(5));
        assert_eq!(p.timeouts().read_idle(), Duration::from_secs(10));
        assert_eq!(p.timeouts().total(), Duration::from_secs(60));
    }

    #[test]
    fn origin_allowlist_is_exact_after_normalization() {
        let p = policy();
        assert!(
            p.authorize_origin(&origin("registry.example.invalid", 443))
                .is_ok()
        );
        assert!(
            p.authorize_origin(&origin("mirror.example.invalid", 8443))
                .is_ok()
        );
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
        assert_eq!(validated.origin(), &registry);
        assert_eq!(validated.addresses().len(), 2);
        assert!(validated.addresses().contains(&ip("8.8.8.8")));
        assert!(validated.addresses().contains(&ip("2606:4700:4700::1111")));
    }

    #[test]
    fn ip_literal_resolution_must_match_the_literal_exactly() {
        let p = policy();
        let direct = origin("8.8.8.8", 443);
        assert!(p.authorize_resolution(&direct, &[ip("8.8.8.8")]).is_ok());
        assert_eq!(
            p.authorize_resolution(&direct, &[ip("1.1.1.1")]),
            Err(PolicyError::IpLiteralResolutionMismatch {
                expected: ip("8.8.8.8"),
            })
        );
        assert_eq!(
            p.authorize_resolution(&direct, &[ip("8.8.8.8"), ip("1.1.1.1")]),
            Err(PolicyError::IpLiteralResolutionMismatch {
                expected: ip("8.8.8.8"),
            })
        );
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
            "5f00::1",
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
        assert!(
            p.verify_connected_peer(&registry, &validated, SocketAddr::new(ip("8.8.8.8"), 443))
                .is_ok()
        );
        assert_eq!(
            p.verify_connected_peer(&registry, &validated, SocketAddr::new(ip("1.1.1.1"), 443)),
            Err(PolicyError::ConnectedPeerNotValidated(ip("1.1.1.1")))
        );
        assert_eq!(
            p.verify_connected_peer(&registry, &validated, SocketAddr::new(ip("127.0.0.1"), 443)),
            Err(PolicyError::NonPublicDestination(ip("127.0.0.1")))
        );
        assert_eq!(
            p.verify_connected_peer(&registry, &validated, SocketAddr::new(ip("8.8.8.8"), 8443)),
            Err(PolicyError::ConnectedPeerPortMismatch {
                expected: 443,
                actual: 8443,
            })
        );
    }

    #[test]
    fn validated_resolution_cannot_be_reused_for_another_origin() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let mirror = origin("mirror.example.invalid", 8443);
        let validated = p.authorize_resolution(&registry, &[ip("8.8.8.8")]).unwrap();

        assert_eq!(
            p.verify_connected_peer(
                &mirror,
                &validated,
                SocketAddr::new(ip("8.8.8.8"), 8443)
            ),
            Err(PolicyError::ResolutionOriginMismatch {
                requested: mirror,
                validated_for: registry,
            })
        );
    }

    #[test]
    fn redirects_require_allowlisted_origins_and_stateful_bounded_hops() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let mirror = origin("mirror.example.invalid", 8443);
        let mut chain = p.start_redirect_chain(&registry).unwrap();

        let same = p.authorize_redirect_hop(&mut chain, &registry).unwrap();
        assert!(same.same_origin_credentials_allowed());
        assert_eq!(chain.redirects_followed(), 1);

        let cross = p.authorize_redirect_hop(&mut chain, &mirror).unwrap();
        assert_eq!(cross.target(), &mirror);
        assert!(!cross.same_origin_credentials_allowed());
        assert_eq!(chain.current_origin(), &mirror);
        assert_eq!(chain.redirects_followed(), 2);

        p.authorize_redirect_hop(&mut chain, &registry).unwrap();
        assert_eq!(chain.redirects_followed(), 3);
        assert_eq!(
            p.authorize_redirect_hop(&mut chain, &mirror),
            Err(PolicyError::RedirectLimitExceeded { limit: 3 })
        );
        assert!(chain.is_failed());
        assert_eq!(
            p.authorize_redirect_hop(&mut chain, &registry),
            Err(PolicyError::RedirectChainFailed)
        );
    }

    #[test]
    fn rejected_redirect_poison_chain() {
        let p = policy();
        let registry = origin("registry.example.invalid", 443);
        let attacker = origin("attacker.example.invalid", 443);
        let mirror = origin("mirror.example.invalid", 8443);
        let mut chain = p.start_redirect_chain(&registry).unwrap();

        assert!(matches!(
            p.authorize_redirect_hop(&mut chain, &attacker),
            Err(PolicyError::OriginNotAllowed(_))
        ));
        assert!(chain.is_failed());
        assert_eq!(
            p.authorize_redirect_hop(&mut chain, &mirror),
            Err(PolicyError::RedirectChainFailed)
        );
    }

    #[test]
    fn redirect_chain_remains_bound_to_its_originating_allowlist() {
        let registry = origin("registry.example.invalid", 443);
        let mirror = origin("mirror.example.invalid", 8443);
        let strict = FetchPolicy::new([registry.clone()], 2, 1024, timeouts()).unwrap();
        let permissive = policy();
        let mut chain = strict.start_redirect_chain(&registry).unwrap();

        assert_eq!(
            permissive.authorize_redirect_hop(&mut chain, &mirror),
            Err(PolicyError::OriginNotAllowed(mirror.clone()))
        );
        assert!(chain.is_failed());
        assert_eq!(
            permissive.authorize_redirect_hop(&mut chain, &registry),
            Err(PolicyError::RedirectChainFailed)
        );
    }

    #[test]
    fn cumulative_download_budget_is_terminal_after_overflow() {
        let p = policy();
        let mut budget = p.start_download();
        assert_eq!(budget.record_chunk(512), Ok(512));
        assert_eq!(budget.record_chunk(512), Ok(1024));
        assert_eq!(budget.received_bytes(), 1024);
        assert!(!budget.is_failed());

        assert_eq!(
            budget.record_chunk(1),
            Err(PolicyError::DownloadLimitExceeded {
                received: 1025,
                limit: 1024,
            })
        );
        assert!(budget.is_failed());
        assert_eq!(budget.received_bytes(), 1025);
        assert_eq!(
            budget.record_chunk(1),
            Err(PolicyError::DownloadLimitExceeded {
                received: 1025,
                limit: 1024,
            })
        );

        let mut near_overflow = DownloadBudget {
            limit: u64::MAX,
            received: u64::MAX - 1,
            failure: None,
        };
        assert_eq!(
            near_overflow.record_chunk(2),
            Err(PolicyError::DownloadCounterOverflow)
        );
        assert!(near_overflow.is_failed());
        assert_eq!(
            near_overflow.record_chunk(0),
            Err(PolicyError::DownloadCounterOverflow)
        );
    }

    #[test]
    fn rejects_unsupported_content_encodings_by_default() {
        assert_eq!(validate_content_encoding_headers(&[]), Ok(()));
        assert_eq!(validate_content_encoding_headers(&["identity"]), Ok(()));
        assert_eq!(validate_content_encoding_headers(&[" IDENTITY "]), Ok(()));
        for encodings in [
            &["gzip"][..],
            &["br"][..],
            &["identity, gzip"][..],
            &[""][..],
            &["identity", "identity"][..],
        ] {
            assert_eq!(
                validate_content_encoding_headers(encodings),
                Err(PolicyError::UnsupportedContentEncoding),
                "{encodings:?}"
            );
        }
    }

    #[test]
    fn content_length_headers_are_unambiguous_decimal_and_bounded() {
        let p = policy();
        assert_eq!(p.validate_content_length_headers(&[]), Ok(None));
        assert_eq!(p.validate_content_length_headers(&["0"]), Ok(Some(0)));
        assert_eq!(
            p.validate_content_length_headers(&[" 1024\t"]),
            Ok(Some(1024))
        );
        assert_eq!(
            p.validate_content_length_headers(&["1025"]),
            Err(PolicyError::DeclaredLengthExceeded {
                declared: 1025,
                limit: 1024,
            })
        );
        assert_eq!(
            p.validate_content_length_headers(&["1024", "1024"]),
            Err(PolicyError::AmbiguousContentLength)
        );
        for value in ["", "+1", "-1", "1, 1", "18446744073709551616", "1\r\n2"] {
            assert_eq!(
                p.validate_content_length_headers(&[value]),
                Err(PolicyError::InvalidContentLength),
                "{value:?}"
            );
        }
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
