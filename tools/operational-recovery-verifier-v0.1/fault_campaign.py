#!/usr/bin/env python3
"""Deterministic synthetic fault campaign for the operational recovery verifier.

This runner builds immutable event traces using the verifier's test fixture
builder, evaluates them with the actual recovery_trace.verify implementation,
and checks outcomes against a versioned fault matrix. It performs no live
host, connector, ticketing or customer mutations.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BENCH = ROOT / "benchmarks" / "operational-recovery-v0.1"
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from recovery_trace import digest_record, verify, verifier_source_digest  # noqa: E402
from test_recovery_trace import Builder, D  # noqa: E402


def _hash_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


def _coordinator_head(builder: Builder) -> dict:
    rows = [e for e in builder.events if e["source_id"] == "platform-coordinator" and e["source_epoch"] == 0]
    if not rows:
        raise AssertionError("scenario has no coordinator epoch-zero event")
    return rows[-1]


def _restart_and_scan(builder: Builder, *, scanned: list[str], unresolved: list[str], reconciled: list[str], include_service_parent: bool = False):
    head = _coordinator_head(builder)
    parents = [head["event_id"]]
    service = builder.by_id.get("verify-service")
    if include_service_parent and service is not None:
        parents.append(service["event_id"])
    restart = builder.add("restart-epoch-1", "coordinator_restarted", parents=tuple(parents), epoch=1)
    scan = builder.add(
        "recovery-scan-epoch-1", "recovery_scan_completed", parents=(restart["event_id"],), epoch=1,
        data={"scanned_attempt_ids": scanned, "unresolved_attempt_ids": unresolved, "reconciled_attempt_ids": reconciled},
    )
    return restart, scan


def _finish_attempt_two(builder: Builder, parent_id: str, *, epoch: int = 0):
    auth = builder.auth("attempt-2", D("d"), parents=(parent_id,))
    dispatch = builder.dispatch("attempt-2", D("d"), parents=(auth["event_id"],), epoch=epoch)
    accepted = builder.accepted("attempt-2", parents=(dispatch["event_id"],))
    effect = builder.effect("attempt-2", parents=(accepted["event_id"],))
    reconcile = builder.reconcile("attempt-2", parents=(effect["event_id"],))
    step = builder.step_verified("attempt-2", parents=(reconcile["event_id"],))
    builder.service_verified(parents=(step["event_id"],))


def _scenarios() -> dict[str, Builder]:
    cases: dict[str, Builder] = {}

    b = Builder(); b.complete_effect(); cases["F001"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); lost = b.lost(parents=(dispatch["event_id"],))
    proof = b.status(outcome="never_accepted", consistency="strong", authoritative=True, parents=(lost["event_id"],))
    _finish_attempt_two(b, proof["event_id"]); cases["F002"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); lost = b.lost(parents=(dispatch["event_id"],))
    proof = b.status(outcome="never_accepted", consistency="eventual", authoritative=True, parents=(lost["event_id"],))
    auth2 = b.auth("attempt-2", D("d"), parents=(proof["event_id"],)); b.dispatch("attempt-2", D("d"), parents=(auth2["event_id"],))
    cases["F003"] = b

    b = Builder(); b.complete_effect()
    b.add("effect-duplicate", "effect_committed", source="itsm-ledger", kind="participant", trust="itsm-control-plane",
          attempt="attempt-1", step="step-a", parents=("accepted-attempt-1",), data={
              "effect_id": "effect-duplicate", "effect_digest": D("b"), "request_digest": D("7"),
              "target_digest": D("6"), "operation_id": "ticket.update", "operation_binding_digest": D("f"),
              "idempotency_key_digest": D("8")})
    cases["F004"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); b.lost(parents=(dispatch["event_id"],))
    _restart_and_scan(b, scanned=[], unresolved=[], reconciled=[]); cases["F005"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); b.lost(parents=(dispatch["event_id"],))
    _, scan = _restart_and_scan(b, scanned=["attempt-1"], unresolved=["attempt-1"], reconciled=[])
    auth2 = b.auth("attempt-2", D("d"), parents=(scan["event_id"],)); b.dispatch("attempt-2", D("d"), parents=(auth2["event_id"],), epoch=1)
    cases["F006"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); b.lost(parents=(dispatch["event_id"],))
    restart = b.add("restart-epoch-1", "coordinator_restarted", parents=(_coordinator_head(b)["event_id"],), epoch=1)
    auth2 = b.auth("attempt-2", D("d"), parents=(restart["event_id"],)); b.dispatch("attempt-2", D("d"), parents=(auth2["event_id"],), epoch=1)
    cases["F007"] = b

    b = Builder(); auth = b.auth(); b.dispatch(parents=(auth["event_id"],)); cases["F008"] = b

    b = Builder(); auth = b.auth(); b.dispatch(parents=(auth["event_id"],))
    b.add("agent-resolved", "agent_declared_resolved", data={"claim": "resolved"}); cases["F009"] = b

    b = Builder(); auth = b.auth(); b.dispatch(grant=D("e"), parents=(auth["event_id"],)); cases["F010"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],))
    dispatch["data"]["durability_attested"] = False
    dispatch["event_digest"] = digest_record(dispatch)
    cases["F011"] = b

    b = Builder(); b.complete_effect(); service = b.by_id["verify-service"]
    service["data"]["valid_until"] = "2026-10-31T10:02:00Z"
    service["event_digest"] = digest_record(service)
    cases["F012"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); b.accepted(parents=(dispatch["event_id"],))
    _restart_and_scan(b, scanned=["attempt-1"], unresolved=["attempt-1"], reconciled=[])
    cases["F013"] = b

    b = Builder(); b.complete_effect()
    _restart_and_scan(b, scanned=["attempt-1"], unresolved=[], reconciled=["attempt-1"], include_service_parent=True)
    cases["F014"] = b

    b = Builder(); auth = b.auth(); dispatch = b.dispatch(parents=(auth["event_id"],)); accepted = b.accepted(parents=(dispatch["event_id"],))
    b.status(outcome="never_accepted", consistency="strong", authoritative=True, parents=(accepted["event_id"],))
    cases["F015"] = b

    return cases


def run_campaign(matrix_path: Path | None = None) -> dict:
    matrix_path = matrix_path or (BENCH / "fault-matrix.json")
    matrix_bytes = matrix_path.read_bytes()
    matrix = json.loads(matrix_bytes)
    if matrix.get("schema") != "luminous-operational-recovery-fault-matrix-v0.1" or matrix.get("version") != 1:
        raise ValueError("unsupported fault matrix schema/version")
    if matrix.get("status") != "research_only":
        raise ValueError("fault campaign must remain research_only")

    scenarios = _scenarios()
    rows = []
    declared_ids = [s.get("scenario_id") for s in matrix.get("scenarios", [])]
    if len(declared_ids) != len(set(declared_ids)):
        raise ValueError("fault matrix contains duplicate scenario IDs")
    if set(declared_ids) != set(scenarios):
        raise ValueError(f"fault matrix/scenario implementation mismatch: declared={sorted(declared_ids)} implemented={sorted(scenarios)}")

    for spec in matrix["scenarios"]:
        builder = scenarios[spec["scenario_id"]]
        result = verify(builder.m, builder.events)
        actual_codes = sorted({v["code"] for v in result.get("violations", [])})
        required_codes = sorted(spec["must_include_violation_codes"])
        missing_codes = sorted(set(required_codes) - set(actual_codes))
        status_match = result["status"] == spec["expected_status"]
        passed = status_match and not missing_codes
        rows.append({
            "scenario_id": spec["scenario_id"], "name": spec["name"],
            "expected_status": spec["expected_status"], "actual_status": result["status"],
            "required_violation_codes": required_codes, "actual_violation_codes": actual_codes,
            "missing_required_violation_codes": missing_codes,
            "event_count": result.get("event_count", len(builder.events)),
            "manifest_digest": result.get("manifest_digest"),
            "event_log_digest": result.get("event_log_digest"),
            "passed": passed,
        })

    report = {
        "schema": "luminous-operational-recovery-fault-campaign-report-v0.1",
        "version": 1,
        "campaign_id": matrix["campaign_id"],
        "status": "pass" if all(r["passed"] for r in rows) else "fail",
        "case_count": len(rows),
        "passed_case_count": sum(r["passed"] for r in rows),
        "failed_case_count": sum(not r["passed"] for r in rows),
        "counts_by_expected_status": {k: sum(r["expected_status"] == k for r in rows) for k in ("valid", "invalid", "indeterminate")},
        "matrix_sha256": _hash_bytes(matrix_bytes),
        "campaign_source_sha256": _hash_bytes(Path(__file__).read_bytes()),
        "verifier_source_sha256": verifier_source_digest(),
        "cases": rows,
        "claim_ceiling": matrix["claim_ceiling"],
    }
    report["report_sha256"] = _hash_bytes(_canonical(report))
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", type=Path, default=BENCH / "fault-matrix.json")
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        report = run_campaign(args.matrix)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            Path(args.output).write_text(rendered, encoding="utf-8")
        return 0 if report["status"] == "pass" else 1
    except (OSError, json.JSONDecodeError, ValueError, AssertionError) as exc:
        print(f"recovery-fault-campaign: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
