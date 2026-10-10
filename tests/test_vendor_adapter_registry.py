"""Adversarial tests for the global vendor adapter capability registry."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import load_json  # noqa: E402
from validate_vendor_adapter_registry import registry_errors  # noqa: E402


class VendorAdapterRegistryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = load_json(ROOT / "security/vendor-adapter-capability-registry-v1.json")
        cls.schema = load_json(ROOT / "schemas/vendor-adapter-capability-registry-v1.schema.json")

    def errors(self, payload):
        return registry_errors(payload, self.schema)

    def test_baseline_is_valid_and_claims_no_implemented_adapters(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.registry)), [])
        self.assertEqual(self.registry["registry_state"], "design_baseline_not_implemented")
        self.assertTrue(all(adapter["maturity"] == "A0_DISCOVERED" for adapter in self.registry["adapters"]))
        self.assertTrue(all(adapter["implementation_state"] == "NOT_IMPLEMENTED" for adapter in self.registry["adapters"]))
        self.assertTrue(all(
            operation["state"] == "planned"
            for adapter in self.registry["adapters"]
            for operation in adapter["operations"]
        ))

    def test_duplicate_adapter_ids_fail(self) -> None:
        payload = copy.deepcopy(self.registry)
        payload["adapters"].append(copy.deepcopy(payload["adapters"][0]))
        self.assertTrue(any("adapter_id duplicates" in error for error in self.errors(payload)))

    def test_duplicate_operation_entries_fail(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        adapter["operations"].append(copy.deepcopy(adapter["operations"][0]))
        self.assertTrue(any("operation duplicates" in error for error in self.errors(payload)))

    def test_mutation_cannot_be_automatically_retried(self) -> None:
        payload = copy.deepcopy(self.registry)
        entry = next(x for x in payload["operation_catalog"] if x["operation"] == "configuration_apply")
        entry["automatic_retry_after_uncertain_outcome"] = True
        self.assertTrue(any("no-blind-replay" in error for error in self.errors(payload)))

    def test_operation_risk_class_cannot_be_downgraded(self) -> None:
        payload = copy.deepcopy(self.registry)
        entry = next(x for x in payload["operation_catalog"] if x["operation"] == "alternative_os_flash")
        entry["risk_class"] = "read_only"
        self.assertTrue(any("risk_class" in error for error in self.errors(payload)))

    def test_a0_cannot_claim_an_implemented_operation(self) -> None:
        payload = copy.deepcopy(self.registry)
        payload["adapters"][0]["operations"][0]["state"] = "tested"
        payload["adapters"][0]["operations"][0].update({
            "evidence_refs": ["evidence:test-run-1"],
            "tested_revision": "a" * 40,
            "tested_scope": {
                "model": "Exact Model",
                "product_family": "IOS XE",
                "hardware_revision": "Rev A",
                "software_version": "1.2.3",
                "region": "test-lab",
                "test_environment": "isolated bench fixture",
            },
        })
        errors = self.errors(payload)
        self.assertTrue(any("A0_DISCOVERED permits planned" in error for error in errors))
        self.assertTrue(any("A0_DISCOVERED must remain NOT_IMPLEMENTED" in error for error in errors))

    def test_tested_operation_requires_exact_scope_revision_and_evidence(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        operation = adapter["operations"][0]
        operation["state"] = "tested"
        errors = self.errors(payload)
        self.assertTrue(any("tested operation requires evidence_refs" in error or "evidence_refs" in error for error in errors))
        self.assertTrue(any("tested_revision" in error for error in errors))
        self.assertTrue(any("tested_scope" in error for error in errors))

    def test_state_change_requires_action_binding_fresh_prestate_and_recovery(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        adapter.update({
            "maturity": "A3_CHANGE_QUALIFIED",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
            "maturity_evidence_refs": ["evidence:adapter-contract-tests"],
        })
        operation = next(x for x in adapter["operations"] if x["operation"] == "configuration_apply")
        operation["state"] = "implemented_untested"
        errors = self.errors(payload)
        self.assertTrue(any("requires security_controls" in error for error in errors))

    def test_state_change_with_insecure_controls_fails(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        adapter.update({
            "maturity": "A3_CHANGE_QUALIFIED",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
            "maturity_evidence_refs": ["evidence:adapter-contract-tests"],
        })
        operation = next(x for x in adapter["operations"] if x["operation"] == "configuration_apply")
        operation.update({
            "state": "implemented_untested",
            "security_controls": {
                "action_digest_bound_authorization": False,
                "fresh_prestate_required": False,
                "uncertain_outcome_policy": "REPLAY_ONLY_AFTER_VERIFIED_IDEMPOTENCY",
                "recovery_evidence_refs": [],
            },
        })
        errors = self.errors(payload)
        self.assertTrue(any("exact action digest" in error for error in errors))
        self.assertTrue(any("fresh pre-state" in error for error in errors))
        self.assertTrue(any("must not trigger automatic replay" in error for error in errors))
        self.assertTrue(any("requires tested recovery evidence" in error for error in errors))

    def test_alternative_os_flash_requires_a4_or_a5_and_tested_recovery(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = next(x for x in payload["adapters"] if x["adapter_id"] == "openwrt-supported-devices")
        adapter.update({
            "maturity": "A3_CHANGE_QUALIFIED",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
            "maturity_evidence_refs": ["evidence:adapter-change-qualification"],
        })
        operation = next(x for x in adapter["operations"] if x["operation"] == "alternative_os_flash")
        operation.update({
            "state": "tested",
            "evidence_refs": ["evidence:flash-test-run"],
            "tested_revision": "b" * 40,
            "tested_scope": {
                "model": "Exact Router",
                "product_family": "Exact router family",
                "hardware_revision": "Rev B",
                "software_version": "OpenWrt target build",
                "region": "test-lab",
                "test_environment": "isolated non-production bench",
            },
            "security_controls": {
                "action_digest_bound_authorization": True,
                "fresh_prestate_required": True,
                "uncertain_outcome_policy": "NEVER_AUTOMATIC_REPLAY",
                "recovery_evidence_refs": ["evidence:physical-recovery-drill"],
            },
        })
        errors = self.errors(payload)
        self.assertTrue(any("requires A4 or A5 maturity" in error for error in errors))

    def test_unsupported_operation_requires_a_reason(self) -> None:
        payload = copy.deepcopy(self.registry)
        op = payload["adapters"][0]["operations"][0]
        op["state"] = "unsupported"
        self.assertTrue(any("rationale" in error for error in self.errors(payload)))

    def test_unknown_operation_fails_closed(self) -> None:
        payload = copy.deepcopy(self.registry)
        payload["adapters"][0]["operations"][0]["operation"] = "arbitrary_shell_exec"
        self.assertTrue(self.errors(payload))

    def test_promotion_requires_evidence(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        adapter.update({
            "maturity": "A1_READ_ONLY_TESTED",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
        })
        errors = self.errors(payload)
        self.assertTrue(any("maturity_evidence_refs" in error for error in errors))
        self.assertTrue(any("tested read-only operation" in error for error in errors))

    def test_a5_requires_region_scope_and_qualified_implementation(self) -> None:
        payload = copy.deepcopy(self.registry)
        adapter = payload["adapters"][0]
        adapter.update({
            "maturity": "A5_SERVICEABLE",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
            "region_scope_state": "not_assessed",
            "maturity_evidence_refs": ["evidence:service-qualification"],
        })
        errors = self.errors(payload)
        self.assertTrue(any("A5_SERVICEABLE requires QUALIFIED implementation" in error for error in errors))
        self.assertTrue(any("requires assessed regional scope" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
