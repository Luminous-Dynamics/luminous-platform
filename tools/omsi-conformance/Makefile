.PHONY: test schemas validate-fixtures interop field-service oca-adapter erpnext-adapter

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

field-service:
	python -c 'import sys,unittest; suite=unittest.defaultTestLoader.discover("tests", pattern="test_field_service.py"); result=unittest.TextTestRunner(verbosity=2).run(suite); sys.exit(0 if result.wasSuccessful() and result.testsRun >= 5 else 1)'

oca-adapter:
	python -c 'import sys,unittest; suite=unittest.defaultTestLoader.discover("tests", pattern="test_oca_fsm_adapter.py"); result=unittest.TextTestRunner(verbosity=2).run(suite); sys.exit(0 if result.wasSuccessful() and result.testsRun >= 23 else 1)'

erpnext-adapter:
	python -c 'import sys,unittest; suite=unittest.defaultTestLoader.discover("tests", pattern="test_erpnext_maintenance_adapter.py"); result=unittest.TextTestRunner(verbosity=2).run(suite); sys.exit(0 if result.wasSuccessful() and result.testsRun >= 47 else 1)'
