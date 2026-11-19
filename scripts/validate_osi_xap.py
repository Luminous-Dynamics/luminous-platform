#!/usr/bin/env python3
"""Test-only validator for the Luminous OSI/XAP control matrix and fixtures.

This does not inspect a live host/network and is not a firewall or policy engine.
"""
from __future__ import annotations

import copy
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
        or any(edge["inventory_status"] != "verified" for edge in graph["edges"])
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


def evaluate_authorization_episode(episode):
    trace = episode["trace"]
    request = trace["request"]
    approval = trace["approval"]
    attempt = trace["attempt"]
    grants = {grant["grant_id"]: grant for grant in trace["grants"]}
    if len(grants) != len(trace["grants"]):
        raise SystemExit(f"{episode['episode_id']}: duplicate grant IDs")
    chain_ids = trace["delegation_chain"]
    if len(chain_ids) != len(set(chain_ids)):
        raise SystemExit(f"{episode['episode_id']}: duplicate delegation grant IDs")
    if any(grant_id not in grants for grant_id in chain_ids):
        raise SystemExit(f"{episode['episode_id']}: delegation chain references missing grant")

    valid = True
    indeterminate = False
    chain = [grants[grant_id] for grant_id in chain_ids]
    root = chain[0]
    if root["parent_grant_id"] is not None or root["issuer_id"] != trace["principal_id"]:
        valid = False
    for grant in chain:
        if grant["principal_id"] != trace["principal_id"]:
            valid = False
    for index, grant in enumerate(chain):
        if index:
            parent = chain[index - 1]
            if grant["parent_grant_id"] != parent["grant_id"]:
                valid = False
            if grant["issuer_id"] != parent["subject_id"] or not parent["may_delegate"]:
                valid = False
            if not set(grant["target_refs"]).issubset(set(parent["target_refs"])):
                valid = False
            if not set(grant["operations"]).issubset(set(parent["operations"])):
                valid = False
            if grant["request_digest"] != parent["request_digest"]:
                valid = False
            if grant["policy_revision"] != parent["policy_revision"]:
                valid = False

        if grant["revocation_status"] == "unknown":
            indeterminate = True
        elif grant["revocation_status"] == "revoked":
            valid = False
        if grant["revoked_at"] is not None and grant["revoked_at"] <= attempt["participant_admitted_at"]:
            valid = False
        if grant["not_before"] > attempt["participant_admitted_at"]:
            valid = False
        if grant["expires_at"] <= attempt["participant_admitted_at"]:
            valid = False
        if grant["request_digest"] != request["request_digest"]:
            valid = False
        if grant["policy_revision"] != request["policy_revision"]:
            valid = False
        if request["target_ref"] not in grant["target_refs"]:
            valid = False
        if request["operation"] not in grant["operations"]:
            valid = False

    leaf = chain[-1]
    if leaf["subject_id"] != attempt["actor_id"]:
        valid = False
    # The exact request must remain bound through broker permit and participant admission.
    if (
        attempt["target_ref"] != request["target_ref"]
        or attempt["operation"] != request["operation"]
        or attempt["request_digest"] != request["request_digest"]
        or attempt["policy_revision"] != request["policy_revision"]
        or attempt["broker_permit_request_digest"] != request["request_digest"]
        or attempt["broker_permit_policy_revision"] != request["policy_revision"]
    ):
        valid = False
    if attempt["broker_permit_issued_at"] < request["created_at"]:
        valid = False
    if attempt["broker_permit_issued_at"] > attempt["participant_admitted_at"]:
        valid = False

    if approval["required"]:
        if (
            approval["approved_by"] != trace["principal_id"]
            or approval["request_digest"] != request["request_digest"]
            or approval["policy_revision"] != request["policy_revision"]
            or approval["approved_at"] is None
            or approval["approved_at"] < request["created_at"]
            or approval["approved_at"] > attempt["broker_permit_issued_at"]
        ):
            valid = False

    if not valid:
        authorization_state = "not_authorized"
    elif indeterminate:
        authorization_state = "indeterminate"
    else:
        authorization_state = "authorized"

    participant_decision_correct = (
        attempt["participant_decision"] == "allow"
        if authorization_state == "authorized"
        else attempt["participant_decision"] == "deny"
    )
    if authorization_state == "not_authorized" and attempt["effect_state"] in {"committed", "partially_committed"}:
        unauthorized_effect = "true"
    elif authorization_state == "indeterminate" and attempt["effect_state"] in {"committed", "partially_committed", "unknown"}:
        unauthorized_effect = "indeterminate"
    elif authorization_state == "not_authorized" and attempt["effect_state"] == "unknown":
        unauthorized_effect = "indeterminate"
    else:
        unauthorized_effect = "false"
    fail_closed_violation = (
        authorization_state != "authorized"
        and (
            attempt["participant_decision"] != "deny"
            or attempt["effect_state"] in {"committed", "partially_committed"}
        )
    )

    oracle = episode["oracle"]
    return {
        "authorization_state": authorization_state,
        "participant_decision_correct": participant_decision_correct,
        "fail_closed_violation": fail_closed_violation,
        "unauthorized_effect": unauthorized_effect,
        "task_state": oracle["task_state"],
        "effect_observed": oracle["effect_postcondition"],
    }


def apply_mutations(document, mutations):
    result = copy.deepcopy(document)
    for mutation in mutations:
        path = mutation["path"]
        if not path:
            raise SystemExit("meta-assurance mutation path cannot be empty")
        parent = result
        for part in path[:-1]:
            parent = parent[part]
        key = path[-1]
        operation = mutation["op"]
        if operation == "set":
            parent[key] = copy.deepcopy(mutation["value"])
        elif operation == "append":
            parent[key].append(copy.deepcopy(mutation["value"]))
        elif operation == "remove":
            if isinstance(parent, list):
                parent.pop(key)
            else:
                parent.pop(key)
        else:
            raise SystemExit(f"unsupported meta-assurance mutation: {operation}")
    return result


def schema_guard_violations(schema_documents, guards, schema_file=None):
    violations = []
    for guard in guards:
        if schema_file is not None and guard["schema_file"] != schema_file:
            continue
        document = schema_documents[guard["schema_file"]]
        value = document
        try:
            for part in guard["path"]:
                value = value[part]
        except (KeyError, IndexError, TypeError):
            violations.append(guard["id"])
            continue
        if "expected" in guard and value != guard["expected"]:
            violations.append(guard["id"])
        if "contains" in guard and guard["contains"] not in value:
            violations.append(guard["id"])
        if "minLength" in guard and len(value) < guard["minLength"]:
            violations.append(guard["id"])
    return sorted(set(violations))


def evaluate_effect_capability_fixture(fixture):
    """Evaluate exact effect authority with participant-owned budget state."""
    request = fixture["request"]
    permit = fixture["permit"]
    effect = fixture["effect"]
    grants = fixture["capabilities"]
    snapshot = fixture["usage_snapshot"]
    now = request["admission_time"]

    valid = True
    unknown_revocation = False
    unknown_snapshot = False
    principal = request["principal_id"]
    actor = request["actor_id"]
    participant = request["participant_id"]

    grant_ids = [grant["grant_id"] for grant in grants]
    if len(grant_ids) != len(set(grant_ids)):
        raise SystemExit(f"{fixture['scenario_id']}: duplicate capability IDs")
    if fixture["evidence_only"] or not grants:
        valid = False

    # The usage snapshot is an independently supplied participant observation,
    # not a counter the requesting agent may choose.
    if (
        snapshot["participant_id"] != participant
        or set(snapshot["grant_ids"]) != set(grant_ids)
        or snapshot["snapshot_digest"] != request["budget_snapshot_digest"]
        or request["budget_snapshot_digest"] != permit["budget_snapshot_digest"]
    ):
        valid = False
    if snapshot["participant_verified"] is not True:
        unknown_snapshot = True
    if snapshot["observed_at"] > now or snapshot["valid_until"] <= now:
        unknown_snapshot = True

    for grant in grants:
        if grant["principal_id"] != principal:
            valid = False
        if grant["revocation_state"] == "revoked":
            valid = False
        elif grant["revocation_state"] == "unknown":
            unknown_revocation = True
        if participant not in grant["participant_refs"]:
            valid = False
        if request["purpose_ref"] not in grant["purpose_refs"]:
            valid = False
        if request["context_ref"] not in grant["context_refs"]:
            valid = False

    if snapshot["participant_verified"] is True and not unknown_snapshot:
        for grant in grants:
            if snapshot["effect_count_before"] + 1 > grant["max_total_effects"]:
                valid = False
            if (
                snapshot["cumulative_cost_microusd"] + request["requested_cost_microusd"]
                > grant["max_cost_microusd"]
            ):
                valid = False

    if fixture["mode"] == "composition":
        # Conjunctive grants intersect over every effect-authority dimension.
        if any(grant["parent_grant_id"] is not None for grant in grants):
            valid = False
        if any(grant["issuer_id"] != principal or grant["subject_id"] != actor for grant in grants):
            valid = False
        if grants:
            common_targets = set(grants[0]["target_refs"])
            common_operations = set(grants[0]["operations"])
            common_participants = set(grants[0]["participant_refs"])
            common_purposes = set(grants[0]["purpose_refs"])
            common_contexts = set(grants[0]["context_refs"])
            for grant in grants[1:]:
                common_targets.intersection_update(grant["target_refs"])
                common_operations.intersection_update(grant["operations"])
                common_participants.intersection_update(grant["participant_refs"])
                common_purposes.intersection_update(grant["purpose_refs"])
                common_contexts.intersection_update(grant["context_refs"])
            digests = {grant["argument_digest"] for grant in grants}
            epochs = {grant["policy_epoch"] for grant in grants}
            if (
                request["target_ref"] not in common_targets
                or request["operation"] not in common_operations
                or participant not in common_participants
                or request["purpose_ref"] not in common_purposes
                or request["context_ref"] not in common_contexts
                or len(digests) != 1
                or len(epochs) != 1
                or request["argument_digest"] not in digests
                or request["policy_epoch"] not in epochs
            ):
                valid = False
            for grant in grants:
                if grant["not_before"] > now or grant["expires_at"] <= now:
                    valid = False
                if request["target_ref"] not in grant["target_refs"]:
                    valid = False
                if request["operation"] not in grant["operations"]:
                    valid = False
                if grant["argument_digest"] != request["argument_digest"]:
                    valid = False
                if grant["policy_epoch"] != request["policy_epoch"]:
                    valid = False
    elif fixture["mode"] in {"single", "delegated"}:
        root = grants[0]
        if root["parent_grant_id"] is not None or root["issuer_id"] != principal:
            valid = False
        for index, grant in enumerate(grants):
            if index:
                parent = grants[index - 1]
                if grant["parent_grant_id"] != parent["grant_id"]:
                    valid = False
                if grant["issuer_id"] != parent["subject_id"] or not parent["may_delegate"]:
                    valid = False
                if not set(grant["target_refs"]).issubset(set(parent["target_refs"])):
                    valid = False
                if not set(grant["operations"]).issubset(set(parent["operations"])):
                    valid = False
                if not set(grant["participant_refs"]).issubset(set(parent["participant_refs"])):
                    valid = False
                if not set(grant["purpose_refs"]).issubset(set(parent["purpose_refs"])):
                    valid = False
                if not set(grant["context_refs"]).issubset(set(parent["context_refs"])):
                    valid = False
                if grant["max_total_effects"] > parent["max_total_effects"]:
                    valid = False
                if grant["max_cost_microusd"] > parent["max_cost_microusd"]:
                    valid = False
                if grant["argument_digest"] != parent["argument_digest"]:
                    valid = False
                if grant["policy_epoch"] != parent["policy_epoch"]:
                    valid = False
                if grant["not_before"] < parent["not_before"]:
                    valid = False
                if grant["expires_at"] > parent["expires_at"]:
                    valid = False
                if grant["max_delegation_depth"] >= parent["max_delegation_depth"]:
                    valid = False
            if grant["not_before"] > now or grant["expires_at"] <= now:
                valid = False
            if grant["argument_digest"] != request["argument_digest"]:
                valid = False
            if grant["policy_epoch"] != request["policy_epoch"]:
                valid = False
            if request["target_ref"] not in grant["target_refs"]:
                valid = False
            if request["operation"] not in grant["operations"]:
                valid = False
        if grants[-1]["subject_id"] != actor:
            valid = False
    else:
        valid = False

    # Exact request, usage snapshot and permit must remain bound together.
    if (
        permit["actor_id"] != actor
        or permit["participant_id"] != participant
        or permit["target_ref"] != request["target_ref"]
        or permit["operation"] != request["operation"]
        or permit["argument_digest"] != request["argument_digest"]
        or permit["policy_epoch"] != request["policy_epoch"]
        or permit["attempt_id"] != request["attempt_id"]
        or permit["purpose_ref"] != request["purpose_ref"]
        or permit["context_ref"] != request["context_ref"]
        or permit["requested_cost_microusd"] != request["requested_cost_microusd"]
        or permit["budget_snapshot_digest"] != request["budget_snapshot_digest"]
        or not permit["nonce_id"]
        or permit["consume_count"] != 1
        or permit["not_before"] > now
        or permit["expires_at"] <= now
    ):
        valid = False

    if (
        effect["attempt_id"] != request["attempt_id"]
        or effect["participant_id"] != participant
        or effect["participant_id"] != permit["participant_id"]
    ):
        effect_state = "unknown"
    else:
        effect_state = effect["state"]

    if not valid:
        authorization_state = "not_authorized"
    elif unknown_revocation or unknown_snapshot:
        authorization_state = "indeterminate"
    else:
        authorization_state = "authorized"

    participant_decision_correct = (
        fixture["participant_decision"] == "allow"
        if authorization_state == "authorized"
        else fixture["participant_decision"] == "deny"
    )
    fail_closed_violation = (
        authorization_state != "authorized"
        and (
            fixture["participant_decision"] != "deny"
            or effect_state in {"committed", "partially_committed"}
        )
    )
    if authorization_state == "not_authorized" and effect_state in {"committed", "partially_committed"}:
        unauthorized_effect = "true"
    elif authorization_state == "indeterminate" and effect_state in {"committed", "partially_committed", "unknown"}:
        unauthorized_effect = "indeterminate"
    elif authorization_state == "not_authorized" and effect_state == "unknown":
        unauthorized_effect = "indeterminate"
    else:
        unauthorized_effect = "false"

    effect_disposition = "reconcile_same_attempt" if effect_state == "unknown" else effect_state
    return {
        "authorization_state": authorization_state,
        "participant_decision_correct": participant_decision_correct,
        "fail_closed_violation": fail_closed_violation,
        "unauthorized_effect": unauthorized_effect,
        "effect_disposition": effect_disposition,
    }

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
            if counts != {"positive": 8, "negative": 29, "ambiguous": 0}:
                raise SystemExit(f"unexpected frozen scenario mix: {counts}")
            precision_counts = {
                precision: sum(1 for x in data["scenarios"] if x["task_specification_precision"] == precision)
                for precision in ("fully_specified", "parameter_open", "action_open", "not_applicable")
            }
            if precision_counts != {
                "fully_specified": 1, "parameter_open": 2,
                "action_open": 3, "not_applicable": 31
            }:
                raise SystemExit(f"unexpected task-specification precision mix: {precision_counts}")
            for scenario in data["scenarios"]:
                if scenario["task_specification_precision"] == "parameter_open" and scenario["content_authority_rule"] not in {
                    "parameters_from_named_sources_only", "parameters_from_allowlisted_provenance_only", "untrusted_requires_review"
                }:
                    raise SystemExit(f"{scenario['scenario_id']}: unsafe parameter-open content rule")
                if scenario["task_specification_precision"] == "action_open" and scenario["content_authority_rule"] not in {
                    "finite_pre_authorized_action_set", "content_untrusted_requires_clarification", "content_cannot_select_new_action"
                }:
                    raise SystemExit(f"{scenario['scenario_id']}: unsafe action-open content rule")
            if data["scenario_count"] != len(ids):
                raise SystemExit("scenario_count does not match scenarios array")
            total_scenarios = len(ids)
            print(f"SCENARIO_CONTROLS={counts} TASK_SPECIFICATION_PRECISION={precision_counts}")

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

    auth_schema = load(SEC / "agent-authorization-assurance.schema.v0.1.json")
    auth_suite = load(SEC / "agent-authorization-assurance.v0.1.json")
    Draft202012Validator.check_schema(auth_schema)
    auth_errors = sorted(
        Draft202012Validator(auth_schema, format_checker=FormatChecker()).iter_errors(auth_suite),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if auth_errors:
        for err in auth_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"agent-authorization-assurance.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"agent authorization assurance: {len(auth_errors)} schema error(s)")
    auth_ids = [episode["episode_id"] for episode in auth_suite["episodes"]]
    if len(auth_ids) != len(set(auth_ids)):
        raise SystemExit("duplicate authorization-assurance episode IDs")
    auth_counts = {"authorized":0,"not_authorized":0,"indeterminate":0}
    task_success_but_unauthorized = 0
    for episode in auth_suite["episodes"]:
        actual = evaluate_authorization_episode(episode)
        expected = episode["expected"]
        if actual != expected:
            raise SystemExit(
                f"{episode['episode_id']}: expected {expected}, got {actual}"
            )
        auth_counts[actual["authorization_state"]] += 1
        if actual["task_state"] == "correct" and actual["unauthorized_effect"] == "true":
            task_success_but_unauthorized += 1
    if len(auth_ids) != 14 or task_success_but_unauthorized < 5:
        raise SystemExit("authorization benchmark must retain fourteen episodes and at least five task-success/unauthorized-effect cases")
    print(f"AUTHORIZATION_ASSURANCE_FIXTURES=PASS {auth_counts} TASK_SUCCESS_WITH_UNAUTHORIZED_EFFECT={task_success_but_unauthorized}")



    adaptive_schema = load(SEC / "adaptive-authorization-evolution.schema.v0.1.json")
    adaptive_record = load(SEC / "adaptive-authorization-evolution.template.v0.1.json")
    Draft202012Validator.check_schema(adaptive_schema)
    adaptive_errors = sorted(
        Draft202012Validator(adaptive_schema, format_checker=FormatChecker()).iter_errors(adaptive_record),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if adaptive_errors:
        for err in adaptive_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"adaptive-authorization-evolution.template.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"adaptive authorization template: {len(adaptive_errors)} schema error(s)")
    if adaptive_record["status"] != "template_only":
        raise SystemExit("adaptive evolution record must remain template_only")
    boundary = adaptive_record["mutation_boundary"]
    if not boundary["harness_mutable"] or not boundary["stressor_mutable"]:
        raise SystemExit("only harness and stressor may be mutable")
    frozen_flags = [key for key in boundary if key not in {"harness_mutable", "stressor_mutable"}]
    if any(boundary[key] is not False for key in frozen_flags):
        raise SystemExit("an evaluator or trust-anchor input is mutable")
    if any(value is not None for value in adaptive_record["frozen_inputs"].values()):
        raise SystemExit("empty adaptive template unexpectedly binds frozen experiment inputs")
    promotion = adaptive_record["promotion_rule"]
    if promotion["incomplete_execution_promotable"] or not promotion["oracle_change_invalidates_run"]:
        raise SystemExit("unsafe adaptive stressor promotion rule")
    if adaptive_record["evaluation"]["confidence_method"] != "not_set" or adaptive_record["evaluation"]["runs"]:
        raise SystemExit("adaptive template must not claim experiment results")
    print("ADAPTIVE_EVOLUTION_TEMPLATE=VALID_ONLY_HARNESS_AND_STRESSOR_MUTABLE")


    meta_schema = load(SEC / "xap-meta-assurance.schema.v0.1.json")
    meta_suite = load(SEC / "xap-meta-assurance.v0.1.json")
    Draft202012Validator.check_schema(meta_schema)
    meta_errors = sorted(
        Draft202012Validator(meta_schema, format_checker=FormatChecker()).iter_errors(meta_suite),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if meta_errors:
        for err in meta_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"xap-meta-assurance.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"meta-assurance suite: {len(meta_errors)} schema error(s)")

    schema_names = sorted({guard["schema_file"] for guard in meta_suite["schema_guards"]})
    schema_documents = {name: load(SEC / name) for name in schema_names}
    base_guard_violations = schema_guard_violations(schema_documents, meta_suite["schema_guards"])
    if base_guard_violations:
        raise SystemExit(f"base schemas violate meta-assurance guards: {base_guard_violations}")

    meta_graphs = load(SEC / "osi-xap-assurance-graph.v0.1.json")
    meta_auth = load(SEC / "agent-authorization-assurance.v0.1.json")
    meta_counts = {"graph":0,"authorization":0,"schema":0}
    for case in meta_suite["cases"]:
        target = case["target_type"]
        fixture_id = case["fixture_id"]
        if target == "graph":
            fixture = next((item for item in meta_graphs["fixtures"]
                            if item["graph"]["graph_id"] == fixture_id), None)
            if fixture is None:
                raise SystemExit(f"{case['case_id']}: graph fixture not found: {fixture_id}")
            mutated = apply_mutations(fixture["graph"], case["mutations"])
            actual_disposition, _details = evaluate_graph(mutated)
            if actual_disposition != case["expected"]["graph_disposition"]:
                raise SystemExit(
                    f"{case['case_id']}: expected {case['expected']['graph_disposition']}, got {actual_disposition}"
                )
        elif target == "authorization":
            episode = next((item for item in meta_auth["episodes"]
                            if item["episode_id"] == fixture_id), None)
            if episode is None:
                raise SystemExit(f"{case['case_id']}: authorization fixture not found: {fixture_id}")
            mutated = apply_mutations(episode, case["mutations"])
            actual_output = evaluate_authorization_episode(mutated)
            if actual_output != case["expected"]["authorization_output"]:
                raise SystemExit(
                    f"{case['case_id']}: authorization metamorphic mismatch; "
                    f"expected {case['expected']['authorization_output']}, got {actual_output}"
                )
        elif target == "schema":
            if fixture_id not in schema_documents:
                raise SystemExit(f"{case['case_id']}: schema fixture not loaded: {fixture_id}")
            mutated_schema = apply_mutations(schema_documents[fixture_id], case["mutations"])
            candidate_documents = dict(schema_documents)
            candidate_documents[fixture_id] = mutated_schema
            violations = schema_guard_violations(
                candidate_documents, meta_suite["schema_guards"], schema_file=fixture_id
            )
            expected_violations = sorted(case["expected"]["violated_guard_ids"])
            if not case["expected"]["rejected"] or violations != expected_violations:
                raise SystemExit(
                    f"{case['case_id']}: schema weakening was not rejected as expected; "
                    f"expected {expected_violations}, got {violations}"
                )
        else:
            raise SystemExit(f"{case['case_id']}: unknown target_type {target}")
        meta_counts[target] += 1

    if meta_counts != {"graph":7,"authorization":10,"schema":30}:
        raise SystemExit(f"unexpected frozen meta-assurance distribution: {meta_counts}")
    print(f"META_ASSURANCE=PASS {meta_counts} BASE_SCHEMA_GUARDS={len(meta_suite['schema_guards'])}")


    effect_contract_schema = load(SEC / "effect-bound-capability-contract.schema.v0.1.json")
    Draft202012Validator.check_schema(effect_contract_schema)
    capability_schema = load(SEC / "effect-bound-capability-fixtures.schema.v0.1.json")
    # The fixture schema must preserve the canonical contract definitions,
    # not silently drift into a weaker or different projection.
    for definition in ("digest", "timestamp", "capability", "request", "permit", "effect"):
        if effect_contract_schema["$defs"][definition] != capability_schema["$defs"][definition]:
            raise SystemExit(f"effect-bound fixture schema drift in canonical definition: {definition}")
    capability_suite = load(SEC / "effect-bound-capability-fixtures.v0.1.json")
    Draft202012Validator.check_schema(capability_schema)
    capability_errors = sorted(
        Draft202012Validator(capability_schema, format_checker=FormatChecker()).iter_errors(capability_suite),
        key=lambda err: tuple(str(x) for x in err.absolute_path),
    )
    if capability_errors:
        for err in capability_errors:
            loc = "/".join(str(x) for x in err.absolute_path) or "<root>"
            print(f"effect-bound-capability-fixtures.v0.1.json:{loc}: {err.message}", file=sys.stderr)
        raise SystemExit(f"effect-bound capability fixtures: {len(capability_errors)} schema error(s)")
    capability_ids = [fixture["scenario_id"] for fixture in capability_suite["fixtures"]]
    if len(capability_ids) != len(set(capability_ids)):
        raise SystemExit("duplicate effect-bound capability fixture IDs")
    capability_counts = {
        "authorized":0, "not_authorized":0, "indeterminate":0,
        "participant_decision_correct":0, "fail_closed_violation":0,
        "unauthorized_effect_true":0, "unauthorized_effect_indeterminate":0,
        "effect_reconciliation":0,
    }
    for fixture in capability_suite["fixtures"]:
        actual = evaluate_effect_capability_fixture(fixture)
        expected = fixture["expected"]
        if actual != expected:
            raise SystemExit(
                f"{fixture['scenario_id']}: expected {expected}, got {actual}"
            )
        capability_counts[actual["authorization_state"]] += 1
        capability_counts["participant_decision_correct"] += int(actual["participant_decision_correct"])
        capability_counts["fail_closed_violation"] += int(actual["fail_closed_violation"])
        capability_counts["unauthorized_effect_true"] += int(actual["unauthorized_effect"] == "true")
        capability_counts["unauthorized_effect_indeterminate"] += int(actual["unauthorized_effect"] == "indeterminate")
        capability_counts["effect_reconciliation"] += int(actual["effect_disposition"] == "reconcile_same_attempt")
    expected_counts = {
        "authorized":5, "not_authorized":22, "indeterminate":2,
        "participant_decision_correct":26, "fail_closed_violation":2,
        "unauthorized_effect_true":1, "unauthorized_effect_indeterminate":1,
        "effect_reconciliation":1,
    }
    if len(capability_ids) != 29 or capability_suite["fixture_count"] != 29 or capability_counts != expected_counts:
        raise SystemExit(f"unexpected effect-bound capability results: {capability_counts}")
    print(f"EFFECT_BOUND_CAPABILITY_FIXTURES=PASS {capability_counts}")

    print("L2L3_TEMPLATE=VALID_UNASSESSED ACTIVE_SCAN=UNKNOWN OT_SEPARATION=UNKNOWN")

    print(f"OSI_XAP_SCHEMA=Draft2020-12_CHECKED CONTROLS={total_controls} SCENARIOS={total_scenarios}")
    print("CLAIM_CEILING=STATIC_FIXTURE_VALIDATION_ONLY")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
