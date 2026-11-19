#!/usr/bin/env python3
"""Test-only validator for the Luminous OSI/XAP control matrix and fixtures.

This does not inspect a live host/network and is not a firewall or policy engine.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SEC = ROOT / "docs" / "security"

def load(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc

def main() -> int:
    files = [
        ("osi-xap-control-matrix.schema.v0.1.json",
         "osi-xap-control-matrix.v0.1.json",
         "luminous-osi-xap-control-matrix"),
        ("osi-xap-scenarios.schema.v0.1.json",
         "osi-xap-scenarios.v0.1.json",
         "luminous-osi-xap-scenarios"),
    ]
    total_controls = 0
    total_scenarios = 0
    for schema_name, data_name, expected_profile in files:
        schema = load(SEC / schema_name)
        data = load(SEC / data_name)
        Draft202012Validator.check_schema(schema)
        errors = sorted(
            Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(data),
            key=lambda err: tuple(str(x) for x in err.absolute_path),
        )
        if errors:
            for err in errors:
                loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
                print(f"{data_name}:{loc}: {err.message}", file=sys.stderr)
            raise SystemExit(f"{data_name}: {len(errors)} schema error(s)")
        if data["profile"] != expected_profile:
            raise SystemExit(f"{data_name}: wrong profile")
        if expected_profile == "luminous-osi-xap-control-matrix":
            layers = [x["layer"] for x in data["osi_layers"]]
            if layers != list(range(1, 8)):
                raise SystemExit("OSI layer inventory must be exactly 1 through 7")
            ids = [c["id"] for c in data["controls"]]
            if len(ids) != len(set(ids)):
                raise SystemExit("duplicate control IDs")
            if data.get("control_count") != len(ids):
                raise SystemExit("control_count does not match controls array")
            for control in data["controls"]:
                if control["osi_layer"] == "cross_layer" and control["x805_security_layer"] != "cross_layer":
                    raise SystemExit(f"{control['id']}: cross-layer control must span the X.805 security layers")
                if control["plane"] == "cross_plane" and control["osi_layer"] != "cross_layer":
                    raise SystemExit(f"{control['id']}: cross_plane control must span OSI layers")
            total_controls = len(ids)
        else:
            ids = [x["scenario_id"] for x in data["scenarios"]]
            if len(ids) != len(set(ids)):
                raise SystemExit("duplicate scenario IDs")
            counts = {kind: sum(1 for x in data["scenarios"] if x["kind"] == kind)
                      for kind in ("positive", "negative", "ambiguous")}
            if counts != {"positive": 5, "negative": 26, "ambiguous": 0}:
                raise SystemExit(f"unexpected frozen scenario mix: {counts}")
            if data["scenario_count"] != len(ids):
                raise SystemExit("scenario_count does not match scenarios array")
            total_scenarios = len(ids)
            print(f"SCENARIO_CONTROLS={counts}")

    # The template is deliberately unqualified. It must not silently become
    # a network inventory or a complete-mediation assertion.
    net_schema = load(SEC / "l2l3-deployment-profile.schema.v0.1.json")
    net_template = load(SEC / "l2l3-deployment-profile.template.v0.1.json")
    Draft202012Validator.check_schema(net_schema)
    net_errors = sorted(
        Draft202012Validator(net_schema, format_checker=FormatChecker()).iter_errors(net_template),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if net_errors:
        for err in net_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"l2l3-deployment-profile.template.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"L2/L3 template: {len(net_errors)} schema error(s)")
    if net_template["status"] != "template_only":
        raise SystemExit("L2/L3 template must remain template_only")
    if net_template["assessment"]["complete_mediation_claim"]:
        raise SystemExit("empty L2/L3 template must not claim complete mediation")
    if net_template["assessment"]["status"] != "not_assessed":
        raise SystemExit("empty L2/L3 template must remain not_assessed")
    if net_template["devices"] or net_template["interfaces"] or net_template["routes"]:
        raise SystemExit("public L2/L3 template must not contain deployment-specific topology")
    if net_template["coverage"]["interface_inventory_complete"] or net_template["coverage"]["ipv4_route_inventory_complete"] or net_template["coverage"]["ipv6_route_inventory_complete"]:
        raise SystemExit("empty L2/L3 template cannot claim inventory completeness")
    if net_template["discovery"]["active_scan_policy"] != "unknown":
        raise SystemExit("empty L2/L3 template must not presume a safe active-scan policy")
    if net_template["ot_network_separation"]["applicability"] != "unknown":
        raise SystemExit("empty L2/L3 template must not presume OT network separation")
    ot = net_template["ot_network_separation"]
    if ot["applicability"] == "applicable":
        zones = {zone["zone_id"]: zone for zone in net_template["zones"]}
        management = set(ot["management_zone_refs"])
        operational = set(ot["operational_zone_refs"])
        monitoring = set(ot["security_monitoring_zone_refs"])
        vendor = set(ot["external_vendor_access_zone_refs"])
        if management & operational:
            raise SystemExit("OT management and operational zones must be distinct")
        if management & monitoring or operational & monitoring:
            raise SystemExit("OT security-monitoring zone must be distinct")
        if vendor & (management | operational):
            raise SystemExit("external vendor access must use a separately declared zone")
        for ref in management:
            if zones.get(ref, {}).get("network_function") != "management":
                raise SystemExit(f"{ref}: expected management zone function")
        for ref in operational:
            if zones.get(ref, {}).get("network_function") != "operational":
                raise SystemExit(f"{ref}: expected operational zone function")
        for ref in monitoring:
            if zones.get(ref, {}).get("network_function") != "security_monitoring":
                raise SystemExit(f"{ref}: expected security_monitoring zone function")
        if ot["telemetry_ingress"] == "controlled_two_way" and not ot["shared_infrastructure_exceptions"]:
            raise SystemExit("two-way OT monitoring path must disclose shared-infrastructure exception")
    # No production network is contacted by this validator. It validates only
    # schema and static fixture semantics; live discovery must use a separately
    # approved read-only/maintenance-window procedure.

    print("L2L3_TEMPLATE=VALID_UNASSESSED ACTIVE_SCAN=UNKNOWN OT_SEPARATION=UNKNOWN")

    print(f"OSI_XAP_SCHEMA=Draft2020-12_CHECKED CONTROLS={total_controls} SCENARIOS={total_scenarios}")
    print("CLAIM_CEILING=STATIC_FIXTURE_VALIDATION_ONLY")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
