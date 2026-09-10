#!/usr/bin/env python3
"""Dependency-free documentation, traceability, and protocol-fixture checks."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REQ = ROOT / "docs" / "requirements.md"
MATRIX = ROOT / "docs" / "verification-matrix.md"
SCHEMA = ROOT / "docs" / "schemas" / "observation-v0.1.schema.json"
FIXTURE = ROOT / "docs" / "fixtures" / "observation-valid-v0.1.json"
REQ_ROW = re.compile(r"^\| ([A-Z]+-\d{3}) \|")
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


class CheckError(Exception):
    pass


def table_ids(path: Path) -> list[str]:
    return [match.group(1) for line in path.read_text(encoding="utf-8").splitlines() if (match := REQ_ROW.match(line))]


def check_traceability() -> int:
    requirements = table_ids(REQ)
    matrix = table_ids(MATRIX)
    if not requirements:
        raise CheckError("no canonical requirement rows found")
    duplicates = sorted({item for item in requirements if requirements.count(item) > 1})
    if duplicates:
        raise CheckError(f"duplicate canonical requirement IDs: {duplicates}")
    matrix_duplicates = sorted({item for item in matrix if matrix.count(item) > 1})
    if matrix_duplicates:
        raise CheckError(f"duplicate matrix IDs: {matrix_duplicates}")
    missing = sorted(set(requirements) - set(matrix))
    unknown = sorted(set(matrix) - set(requirements))
    if missing or unknown:
        raise CheckError(f"matrix mismatch: missing={missing}, unknown={unknown}")
    return len(requirements)


def check_local_links() -> tuple[int, int]:
    excluded_parts = {"node_modules", "dist", "target", ".git"}
    markdown_files = sorted(
        document
        for document in ROOT.rglob("*.md")
        if not excluded_parts.intersection(document.relative_to(ROOT).parts[:-1])
    )
    checked = 0
    failures: list[str] = []
    for document in markdown_files:
        text = document.read_text(encoding="utf-8")
        for raw_target in LINK.findall(text):
            target = raw_target.strip().split(maxsplit=1)[0].strip("<>")
            if not target or target.startswith(("#", "http://", "https://", "mailto:")):
                continue
            path_part = target.split("#", 1)[0]
            if not path_part:
                continue
            checked += 1
            resolved = (document.parent / path_part).resolve()
            if ROOT not in resolved.parents and resolved != ROOT:
                failures.append(f"{document.relative_to(ROOT)}: link escapes repository: {target}")
            elif not resolved.exists():
                failures.append(f"{document.relative_to(ROOT)}: missing local target: {target}")
    if failures:
        raise CheckError("local link failures:\n" + "\n".join(failures))
    return len(markdown_files), checked


def json_type_matches(expected: str, value: Any) -> bool:
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "null":
        return value is None
    raise CheckError(f"validator does not support JSON type {expected!r}")


def validate(schema: dict[str, Any], value: Any, path: str = "$") -> None:
    if "const" in schema and value != schema["const"]:
        raise CheckError(f"{path}: expected constant {schema['const']!r}, got {value!r}")
    if "enum" in schema and value not in schema["enum"]:
        raise CheckError(f"{path}: {value!r} not in {schema['enum']!r}")

    declared = schema.get("type")
    if declared is not None:
        choices = [declared] if isinstance(declared, str) else declared
        if not any(json_type_matches(choice, value) for choice in choices):
            raise CheckError(f"{path}: expected type {choices!r}, got {type(value).__name__}")

    if isinstance(value, dict):
        required = schema.get("required", [])
        missing = sorted(set(required) - set(value))
        if missing:
            raise CheckError(f"{path}: missing required keys {missing}")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            extras = sorted(set(value) - set(properties))
            if extras:
                raise CheckError(f"{path}: unexpected keys {extras}")
        for key, item in value.items():
            if key in properties:
                validate(properties[key], item, f"{path}.{key}")

    if isinstance(value, list):
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            raise CheckError(f"{path}: too many items")
        item_schema = schema.get("items")
        if item_schema:
            for index, item in enumerate(value):
                validate(item_schema, item, f"{path}[{index}]")

    if isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            raise CheckError(f"{path}: string is too short")
        if "maxLength" in schema and len(value) > schema["maxLength"]:
            raise CheckError(f"{path}: string is too long")
        if "pattern" in schema and re.fullmatch(schema["pattern"], value) is None:
            raise CheckError(f"{path}: does not match {schema['pattern']!r}")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            raise CheckError(f"{path}: below minimum {schema['minimum']}")
        if "maximum" in schema and value > schema["maximum"]:
            raise CheckError(f"{path}: above maximum {schema['maximum']}")


def check_schema_fixture() -> None:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise CheckError("observation schema is not JSON Schema Draft 2020-12")
    validate(schema, fixture)
    if fixture["quality"]["mass"]["uncertainty_g"] is None and "uncertainty_not_characterized" not in fixture["faults"]:
        raise CheckError("null uncertainty must be explicit in faults")


def main() -> int:
    try:
        count = check_traceability()
        markdown_count, link_count = check_local_links()
        check_schema_fixture()
    except (CheckError, json.JSONDecodeError, OSError) as error:
        print(f"CHECK_DOCS: FAIL: {error}", file=sys.stderr)
        return 1
    print(f"traceability: PASS ({count} canonical requirements, complete matrix)")
    print(f"markdown links: PASS ({link_count} local links across {markdown_count} files)")
    print("protocol schema: PASS (Draft 2020-12 JSON parses)")
    print("protocol fixture: PASS (observation-valid-v0.1.json conforms)")
    print("CHECK_DOCS: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
