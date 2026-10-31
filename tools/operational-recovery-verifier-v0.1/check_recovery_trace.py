#!/usr/bin/env python3
"""Structural and semantic smoke check for the committed recovery trace fixture."""
import json
import sys
from pathlib import Path
from recovery_trace import TraceError, verify

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "benchmarks" / "operational-recovery-v0.1"

def main():
    try:
        manifest = json.loads((FIXTURES / "run-manifest.example.json").read_text(encoding="utf-8"))
        events = []
        with (FIXTURES / "events.example.jsonl").open(encoding="utf-8") as stream:
            for line_no, line in enumerate(stream, 1):
                if line.strip():
                    try:
                        events.append(json.loads(line))
                    except json.JSONDecodeError as exc:
                        raise TraceError(f"events.example.jsonl line {line_no}: {exc}") from exc
        report = verify(manifest, events)
        if report["status"] != "valid":
            raise TraceError(f"example trace did not validate: {report['status']} {report['violations']}")
        print(f"PASS causal recovery trace: {report['event_count']} events; {report['required_step_count']} required steps")
        print(f"Manifest SHA256: {report['manifest_digest']}")
        print(f"Event-log SHA256: {report['event_log_digest']}")
        print("SCOPE: record consistency only; not signature, source authenticity, fsync, observer independence or service qualification.")
        return 0
    except (OSError, json.JSONDecodeError, TraceError) as exc:
        print(f"FAIL recovery trace: {exc}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    raise SystemExit(main())
