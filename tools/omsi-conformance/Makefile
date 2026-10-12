.PHONY: test schemas validate-fixtures interop

test:
	python -m unittest discover -s tests -v

schemas:
	python -c 'import json; from jsonschema import Draft202012Validator; from pathlib import Path; [Draft202012Validator.check_schema(json.loads(p.read_text())) for p in Path("schemas/0.1").glob("*.json")]'

validate-fixtures:
	python omsi_validate.py operation fixtures/operation.json --now 2026-10-12T12:05:00Z
	python omsi_validate.py grant fixtures/grant.json --manifest fixtures/operation.json --now 2026-10-12T12:05:00Z
	python omsi_validate.py receipt fixtures/receipt.json --manifest fixtures/operation.json --events fixtures/events.json
	python omsi_validate.py events fixtures/events.json --manifest fixtures/operation.json

interop:
	node interop/verify-digests.mjs
