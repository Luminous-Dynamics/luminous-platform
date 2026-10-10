"""Adversarial tests for endpoint identity, transport and credential boundaries."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import load_json  # noqa: E402
from validate_vendor_management_endpoint_profile import profile_errors  # noqa: E402


class VendorManagementEndpointProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = load_json(ROOT / "tests/fixtures/vendor-management-endpoint-profile-v1.json")
        cls.schema = load_json(ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json")

    def errors(self, profile):
        return profile_errors(profile, self.schema)

    def test_synthetic_baseline_is_valid_draft_only(self) -> None:
        profile = copy.deepcopy(self.fixture)
        self.assertEqual(self.errors(profile), [])
        self.assertEqual(profile["state"], "draft")
        self.assertEqual(profile["authentication"]["read_only"], True)
        self.assertEqual(profile["authentication"]["mutation_enabled"], False)
        self.assertEqual(profile["endpoint"]["host"], "api.example.invalid")

    def test_endpoint_host_cannot_be_uppercased(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["host"] = "API.EXAMPLE.INVALID"
        self.assertTrue(self.errors(profile))

    def test_invalid_dotted_decimal_ip_is_rejected(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["host"] = "999.999.999.999"
        self.assertTrue(any("invalid dotted-decimal IP" in error for error in self.errors(profile)))

    def test_literal_ipv6_address_is_supported(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["host"] = "2001:db8::1"
        self.assertEqual(self.errors(profile), [])

    def test_endpoint_kind_cannot_claim_a_different_transport(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["transport"] = "ssh"
        self.assertTrue(any("transport must match" in error for error in self.errors(profile)))

    def test_tls_hostname_validation_cannot_be_disabled(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["peer_verification"]["hostname_validation"] = False
        self.assertTrue(any("hostname_validation must be true" in error for error in self.errors(profile)))

    def test_tls_minimum_cannot_be_weakened(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["peer_verification"]["minimum_tls_version"] = None
        self.assertTrue(any("minimum_tls_version must be TLSv1.2 or TLSv1.3" in error for error in self.errors(profile)))

    def test_insecure_fallback_is_forbidden_by_schema(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["peer_verification"]["insecure_fallback_allowed"] = True
        self.assertTrue(self.errors(profile))

    def test_mutation_cannot_be_enabled_in_read_only_profile(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["authentication"]["mutation_enabled"] = True
        self.assertTrue(self.errors(profile))

    def test_credential_material_is_not_accepted_as_reference(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["authentication"]["credential_reference"] = "actual-secret-value-123456"
        self.assertTrue(any("$.authentication.credential_reference" in error for error in self.errors(profile)))

    def test_principal_must_match_declared_scope(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["authentication"]["expected_principal_ref"] = "principal:another-identity"
        self.assertTrue(any("must match scope.expected_principal_ref" in error for error in self.errors(profile)))

    def test_tenant_or_org_scope_must_match_exactly(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["authentication"]["expected_tenant_or_org_ref"] = "scope:another-org"
        self.assertTrue(any("must match scope.expected_tenant_or_org_ref" in error for error in self.errors(profile)))

    def test_deny_redirect_policy_rejects_origins(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["redirect_policy"]["allowed_origins"] = ["https://api.example.invalid"]
        self.assertTrue(any("must be empty when mode=deny" in error for error in self.errors(profile)))

    def test_allowlisted_redirects_require_https_origin_only(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["redirect_policy"]["mode"] = "allowlisted_origins"
        profile["redirect_policy"]["allowed_origins"] = ["http://redirect.example.invalid/"]
        profile["limits"]["max_redirects"] = 1
        self.assertTrue(any("must use https" in error for error in self.errors(profile)))

    def test_allowlisted_redirect_origin_must_be_canonical(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["redirect_policy"]["mode"] = "allowlisted_origins"
        profile["redirect_policy"]["allowed_origins"] = ["https://redirect.example.invalid:443"]
        profile["limits"]["max_redirects"] = 1
        self.assertTrue(any("canonical origin syntax" in error for error in self.errors(profile)))

    def test_ipv6_address_must_be_canonical(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["host"] = "2001:0db8:0:0::1"
        self.assertTrue(any("canonical textual representation" in error for error in self.errors(profile)))

    def test_allowlisted_redirect_cannot_contain_path_or_userinfo(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["redirect_policy"]["mode"] = "allowlisted_origins"
        profile["redirect_policy"]["allowed_origins"] = ["https://user:pass@redirect.example.invalid/path"]
        profile["limits"]["max_redirects"] = 1
        errors = self.errors(profile)
        self.assertTrue(any("no user information" in error for error in errors))
        self.assertTrue(any("origin only" in error for error in errors))

    def test_api_path_rejects_encoded_traversal(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["api_base_path"] = "/api/%2e%2e/admin"
        self.assertTrue(any("percent-encoding" in error for error in self.errors(profile)))

    def test_api_path_rejects_ambiguous_trailing_slash(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["api_base_path"] = "/api/v1/"
        self.assertTrue(any("trailing slash" in error for error in self.errors(profile)))

    def test_non_http_transport_cannot_carry_an_api_path(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"]["kind"] = "netconf_ssh"
        profile["endpoint"]["transport"] = "ssh"
        profile["endpoint"]["api_base_path"] = "/api/v1"
        profile["peer_verification"]["method"] = "ssh_known_hosts"
        profile["peer_verification"]["minimum_tls_version"] = None
        profile["authentication"]["mode"] = "ssh_key_reference"
        self.assertTrue(any("must be null for non-HTTP API transports" in error for error in self.errors(profile)))

    def test_read_only_ssh_profile_requires_known_hosts(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["endpoint"].update({"kind": "netconf_ssh", "transport": "ssh", "api_base_path": None})
        profile["authentication"]["mode"] = "ssh_key_reference"
        profile["peer_verification"]["minimum_tls_version"] = None
        profile["peer_verification"]["method"] = "platform_trust_store"
        profile["peer_verification"]["hostname_validation"] = False
        self.assertTrue(any("ssh_known_hosts" in error for error in self.errors(profile)))

    def test_ambient_proxy_is_forbidden(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["egress"]["ambient_proxy_allowed"] = True
        self.assertTrue(self.errors(profile))

    def test_request_timeout_must_cover_connect_timeout(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["limits"]["request_timeout_ms"] = 1000
        profile["limits"]["connect_timeout_ms"] = 5000
        self.assertTrue(any("at least connect_timeout_ms" in error for error in self.errors(profile)))

    def test_assessed_jurisdiction_requires_evidence(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["scope"]["deployment_jurisdiction"] = {
            "state": "assessed",
            "value": "ZZ",
            "rationale": "Synthetic value for adversarial test.",
            "evidence_refs": [],
        }
        self.assertTrue(any("evidence_refs is required when state=assessed" in error for error in self.errors(profile)))

    def test_profile_cannot_self_assert_lab_approval(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["state"] = "approved_for_lab"
        self.assertTrue(any("$.state" in error for error in self.errors(profile)))

    def test_approval_evidence_cannot_be_embedded_as_profile_authority(self) -> None:
        profile = copy.deepcopy(self.fixture)
        profile["approval_evidence_refs"] = ["evidence:lab-approval"]
        self.assertTrue(any("approval_evidence_refs" in error for error in self.errors(profile)))

    def test_draft_profile_can_record_unresolved_scope_without_claiming_approval(self) -> None:
        profile = copy.deepcopy(self.fixture)
        self.assertEqual(profile["state"], "draft")
        self.assertEqual(self.errors(profile), [])


if __name__ == "__main__":
    unittest.main()
