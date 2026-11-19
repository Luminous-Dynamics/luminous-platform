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


def evaluate_graph(graph):
    nodes = {node["id"]: node for node in graph["nodes"]}
    if len(nodes) != len(graph["nodes"]):
        raise SystemExit(f"{graph['graph_id']}: duplicate node IDs")
    edge_ids = [edge["id"] for edge in graph["edges"]]
    if len(edge_ids) != len(set(edge_ids)):
        raise SystemExit(f"{graph['graph_id']}: duplicate edge IDs")

    outgoing = {node_id: [] for node_id in nodes}
    for edge in graph["edges"]:
        if edge["from"] not in nodes or edge["to"] not in nodes:
            raise SystemExit(f"{graph['graph_id']}: edge {edge['id']} has an unknown endpoint")
        outgoing[edge["from"]].append(edge)

    inventory = graph["inventory"]
    has_unknown = (
        not inventory["complete"]
        or bool(inventory["unknown_nodes"])
        or bool(inventory["unknown_edges"])
        or any(edge["inventory_status"] == "unknown" for edge in graph["edges"])
    )
    claim = graph["claim"]

    if claim["property"] == "complete_mediation":
        required = set(claim["required_enforcement_roles"])
        bypasses = []
        unreachable = []
        for source in claim["source_node_refs"]:
            if source not in nodes or nodes[source]["type"] != "untrusted_ingress":
                raise SystemExit(f"{graph['graph_id']}: invalid untrusted source reference {source}")
            for sink in claim["sink_node_refs"]:
                if sink not in nodes or nodes[sink]["type"] != "consequential_sink":
                    raise SystemExit(f"{graph['graph_id']}: invalid consequential sink reference {sink}")
                # State is (node, required enforcement roles already crossed).
                # This explores every distinct enforcement-state path, not only
                # one convenient shortest path.
                start_roles = required.intersection(nodes[source]["enforcement_roles"])
                queue = [(source, frozenset(start_roles))]
                seen = set(queue)
                has_any_path = False
                has_mediated_path = False
                has_unmediated_path = False
                cursor = 0
                while cursor < len(queue):
                    current, roles = queue[cursor]
                    cursor += 1
                    if current == sink:
                        has_any_path = True
                        if required.issubset(roles):
                            has_mediated_path = True
                        else:
                            has_unmediated_path = True
                        continue
                    for edge in outgoing[current]:
                        next_node = edge["to"]
                        next_roles = roles.union(required.intersection(nodes[next_node]["enforcement_roles"]))
                        state = (next_node, frozenset(next_roles))
                        if state not in seen:
                            seen.add(state)
                            queue.append(state)
                if not has_any_path:
                    unreachable.append(f"{source}->{sink}")
                if has_unmediated_path:
                    bypasses.append(f"{source}->{sink}")
                if has_any_path and not has_mediated_path and not has_unmediated_path:
                    unreachable.append(f"{source}->{sink}: no qualified route")
        if bypasses:
            return "refuted", {"bypass_paths": bypasses, "unreachable": unreachable}
        if has_unknown or unreachable:
            return "indeterminate", {"unknown_inventory": has_unknown, "unreachable": unreachable}
        return "qualified", {"bypass_paths": [], "unreachable": []}

    # Independence is tested against transitive shared fault domains and
    # declared dependency nodes. Different observer names/operators alone
    # never establish independence.
    observers = claim["observer_node_refs"]
    if any(ref not in nodes or nodes[ref]["type"] != "observer" for ref in observers):
        raise SystemExit(f"{graph['graph_id']}: invalid observer node reference")
    dependency_sets = {}
    for observer in observers:
        reached = {observer}
        queue = [observer]
        domains = set()
        if nodes[observer]["fault_domain"]:
            domains.add(nodes[observer]["fault_domain"])
        cursor = 0
        while cursor < len(queue):
            current = queue[cursor]
            cursor += 1
            for edge in outgoing[current]:
                if edge["type"] != "depends_on":
                    continue
                target = edge["to"]
                if target not in reached:
                    reached.add(target)
                    queue.append(target)
                if nodes[target]["fault_domain"]:
                    domains.add(nodes[target]["fault_domain"])
        dependency_sets[observer] = reached.union({f"fault-domain:{domain}" for domain in domains})
    overlaps = []
    for index, left in enumerate(observers):
        for right in observers[index + 1:]:
            shared = sorted(dependency_sets[left].intersection(dependency_sets[right]))
            if shared:
                overlaps.append({"observers":[left, right], "shared_dependencies":shared})
    if overlaps:
        return "refuted", {"overlaps":overlaps}
    if has_unknown:
        return "indeterminate", {"unknown_inventory":True,"overlaps":[]}
    return "qualified", {"overlaps":[]}

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


    graph_schema = load(SEC / "osi-xap-assurance-graph.schema.v0.1.json")
    graph_suite = load(SEC / "osi-xap-assurance-graph.v0.1.json")
    Draft202012Validator.check_schema(graph_schema)
    graph_errors = sorted(
        Draft202012Validator(graph_schema, format_checker=FormatChecker()).iter_errors(graph_suite),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if graph_errors:
        for err in graph_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"osi-xap-assurance-graph.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"XAP assurance graph: {len(graph_errors)} schema error(s)")
    graph_results = []
    for fixture in graph_suite["fixtures"]:
        actual, details = evaluate_graph(fixture["graph"])
        expected = fixture["expected_disposition"]
        if actual != expected:
            raise SystemExit(
                f"{fixture['graph']['graph_id']}: expected {expected}, got {actual}; details={details}"
            )
        graph_results.append(actual)
    graph_counts = {value: graph_results.count(value) for value in ("qualified", "refuted", "indeterminate")}
    if graph_counts != {"qualified":2,"refuted":2,"indeterminate":1}:
        raise SystemExit(f"unexpected XAP assurance graph fixture mix: {graph_counts}")
    print(f"XAP_GRAPH_FIXTURES=PASS {graph_counts}")

    print("L2L3_TEMPLATE=VALID_UNASSESSED ACTIVE_SCAN=UNKNOWN OT_SEPARATION=UNKNOWN")

    print(f"OSI_XAP_SCHEMA=Draft2020-12_CHECKED CONTROLS={total_controls} SCENARIOS={total_scenarios}")
    print("CLAIM_CEILING=STATIC_FIXTURE_VALIDATION_ONLY")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
