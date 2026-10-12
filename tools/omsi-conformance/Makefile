.PHONY: test schemas

test:
	python -m unittest discover -s tests -v

schemas:
	python -c 'import json; from jsonschema import Draft202012Validator; from pathlib import Path; [Draft202012Validator.check_schema(json.loads(p.read_text())) for p in Path("schemas/0.1").glob("*.json")]'
