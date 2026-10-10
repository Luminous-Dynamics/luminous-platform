#!/usr/bin/env python3
"""Validate a hardware deployment-profile declaration against its schema and catalog."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json


def profile_errors(
    profile: dict[str, Any],
    schema: dict[str, Any],
    catalog: dict[str, Any],
) -> list[str]:
    """Return schema and cross-document invariant violations."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(validator.iter_errors(profile), key=lambda item: list(map(str, item.path))):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")
    if errors:
        return errors

    catalog_items = {item["id"]: item for item in catalog.get("items", []) if isinstance(item, dict) and "id" in item}
    component_ids: set[str] = set()
    for index, component in enumerate(profile["components"]):
        prefix = f"$.components[{index}]"
        component_id = component["component_id"]
        if component_id in component_ids:
            errors.append(f"{prefix}.component_id duplicates {component_id!r}")
        component_ids.add(component_id)

        missing = [ref for ref in component["catalog_refs"] if ref not in catalog_items]
        if missing:
            errors.append(f"{prefix}.catalog_refs unknown: {', '.join(missing)}")
            continue
        referenced = [catalog_items[ref] for ref in component["catalog_refs"]]

        if component["selection_state"] == "candidate_not_approved":
            candidates = [entry for entry in referenced if entry["maturity"] == "H0_CANDIDATE"]
            if not candidates:
                errors.append(f"{prefix}: candidate_not_approved must reference at least one H0_CANDIDATE entry")
            if all(entry["maturity"] in {"GAP_UNSELECTED", "REFERENCE_ONLY"} for entry in referenced):
                errors.append(f"{prefix}: capability gap/reference cannot be represented as an actual hardware candidate")
        elif component["selection_state"] == "unselected":
            if not any(entry["maturity"] == "GAP_UNSELECTED" for entry in referenced):
                errors.append(f"{prefix}: unselected component must reference an explicit GAP_UNSELECTED catalog entry")

    segment_ids: set[str] = set()
    vlan_ids: set[int] = set()
    segments_by_id: dict[str, dict[str, Any]] = {}
    for index, segment in enumerate(profile["network"]["segments"]):
        prefix = f"$.network.segments[{index}]"
        segment_id = segment["segment_id"]
        vlan_id = segment["vlan_id"]
        if segment_id in segment_ids:
            errors.append(f"{prefix}.segment_id duplicates {segment_id!r}")
        if vlan_id in vlan_ids:
            errors.append(f"{prefix}.vlan_id duplicates {vlan_id}")
        segment_ids.add(segment_id)
        vlan_ids.add(vlan_id)
        segments_by_id[segment_id] = segment

    flow_ids: set[str] = set()
    for index, flow in enumerate(profile["network"]["required_flow_contracts"]):
        prefix = f"$.network.required_flow_contracts[{index}]"
        if flow["id"] in flow_ids:
            errors.append(f"{prefix}.id duplicates {flow['id']!r}")
        flow_ids.add(flow["id"])
        for field in ("from", "to"):
            for zone in flow[field]:
                if zone in segments_by_id:
                    continue
                if zone in {"approved WAN egress", "site networks", "named services"}:
                    continue
                errors.append(f"{prefix}.{field} references unknown segment or named zone {zone!r}")

    assumptions = profile["assumptions"]
    if not (
        assumptions["managed_endpoints_min"]
        <= assumptions["managed_endpoints_target"]
        <= assumptions["managed_endpoints_max"]
    ):
        errors.append("$.assumptions endpoint sizing must satisfy min <= target <= max")

    guest = segments_by_id.get("guest")
    if guest is not None:
        guest_flow = next(
            (flow for flow in profile["network"]["required_flow_contracts"] if flow["id"] == "guest-internet-only"),
            None,
        )
        if not guest["internet_egress"]:
            errors.append("$.network.segments[guest].internet_egress must allow approved outbound access for this guest-only segment")
        if guest_flow is None or guest_flow["decision"] != "allow":
            errors.append("$.network.required_flow_contracts must define an explicit guest-internet-only flow")
        elif not any("internal" in limit.lower() or "site-private" in limit.lower() for limit in guest_flow["limits"]):
            errors.append("$.network guest flow lacks explicit internal-prefix denial constraints")

    if profile["network"]["inter_vlan_default_policy"] != "deny":
        errors.append("$.network.inter_vlan_default_policy must remain deny")
    if profile["network"]["management_plane"]["internet_exposure"] is not False:
        errors.append("$.network.management_plane must not expose management directly to the Internet")
    if profile["network"]["management_plane"]["cloud_controller_allowed"] is True:
        errors.append("$.network.management_plane cloud control is not approved in this baseline")

    gates = profile["acceptance_gates"]
    gate_ids: set[str] = set()
    for index, gate in enumerate(gates):
        prefix = f"$.acceptance_gates[{index}]"
        if gate["gate_id"] in gate_ids:
            errors.append(f"{prefix}.gate_id duplicates {gate['gate_id']!r}")
        gate_ids.add(gate["gate_id"])
        # The profile has not been qualified. Its first revision must not promote
        # an authored test/evidence expectation to a PASS without an evidence field.
        if gate["status"] == "PASS":
            errors.append(f"{prefix}.status cannot be PASS until a versioned evidence reference and exact tested scope are recorded")

    price = profile["pricing_snapshot"]
    minimum_sum = 0.0
    maximum_sum = 0.0
    priced_components: set[str] = set()
    for index, row in enumerate(price["price_research"]):
        prefix = f"$.pricing_snapshot.price_research[{index}]"
        if row["component_id"] in priced_components:
            errors.append(f"{prefix}.component_id duplicates priced component {row['component_id']!r}")
        priced_components.add(row["component_id"])
        if row["catalog_ref"] not in catalog_items:
            errors.append(f"{prefix}.catalog_ref unknown: {row['catalog_ref']!r}")
        component = next((item for item in profile["components"] if item["component_id"] == row["component_id"]), None)
        if component is None:
            errors.append(f"{prefix}.component_id does not match a declared component")
        elif row["catalog_ref"] not in component["catalog_refs"]:
            errors.append(f"{prefix}.catalog_ref is not one of the component's catalog references")
        if row["maximum_zar"] < row["minimum_zar"]:
            errors.append(f"{prefix}.maximum_zar is less than minimum_zar")
    for included_id in price["included_components"]:
        component = next((item for item in profile["components"] if item["component_id"] == included_id), None)
        if component is None:
            errors.append(f"$.pricing_snapshot.included_components unknown: {included_id!r}")
            continue
        row = next((item for item in price["price_research"] if item["component_id"] == included_id), None)
        if row is None:
            errors.append(f"$.pricing_snapshot includes unpriced component {included_id!r}")
            continue
        minimum_sum += row["minimum_zar"] * component["quantity"]
        maximum_sum += row["maximum_zar"] * component["quantity"]
    if round(minimum_sum, 2) != round(price["partial_subtotal_min_zar"], 2):
        errors.append(
            "$.pricing_snapshot.partial_subtotal_min_zar must equal the explicitly included, priced component lower bounds"
        )
    if round(maximum_sum, 2) != round(price["partial_subtotal_max_zar"], 2):
        errors.append(
            "$.pricing_snapshot.partial_subtotal_max_zar must equal the explicitly included, priced component upper bounds"
        )

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, default=Path("profiles/sa-single-site-office-lab-v1.json"))
    parser.add_argument("--profile-schema", type=Path, default=Path("schemas/hardware-deployment-profile-v1.schema.json"))
    parser.add_argument("--catalog", type=Path, default=Path("hardware/portfolio-v1.json"))
    parser.add_argument("--catalog-schema", type=Path, default=Path("schemas/hardware-portfolio-v1.schema.json"))
    args = parser.parse_args(argv)

    try:
        profile = load_json(args.profile)
        schema = load_json(args.profile_schema)
        catalog = load_json(args.catalog)
        catalog_schema = load_json(args.catalog_schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not all(isinstance(item, dict) for item in (profile, schema, catalog, catalog_schema)):
        print("FAIL: profile, schema and catalog roots must be JSON objects", file=sys.stderr)
        return 1

    # Validate the source catalog first; profile references must not bless a malformed catalog.
    from validate_hardware_portfolio import catalog_errors

    catalog_failures = catalog_errors(catalog, catalog_schema)
    if catalog_failures:
        for failure in catalog_failures:
            print(f"FAIL: catalog: {failure}", file=sys.stderr)
        return 1

    errors = profile_errors(profile, schema, catalog)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(
        "PASS: deployment profile and cross-references are structurally valid; "
        "the hardware remains unqualified and deployment is not authorized."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
