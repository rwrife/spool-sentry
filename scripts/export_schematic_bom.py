#!/usr/bin/env python3
"""Export BOM artifacts from a KiCad schematic netlist.

This script keeps KiCad symbol properties as the source of truth and emits:
- docs/verification/spool-sentry.xml (KiCad XML netlist)
- docs/verification/component-properties.csv (one row per component)
- bom/bom.csv (grouped physical BOM; test points excluded)
- docs/verification/bom-analysis.json (coverage/missing-field summary)
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
from collections import OrderedDict
from datetime import datetime, timezone
from pathlib import Path
import xml.etree.ElementTree as ET

REQUIRED_COLUMNS = [
    "Reference",
    "Value",
    "Manufacturer",
    "MPN",
    "Footprint",
    "Datasheet",
    "Supplier",
    "Supplier PN",
    "Estimated Unit Cost USD",
    "Price Observed UTC",
    "Stock Observed",
    "Lifecycle Status",
    "Lifecycle/Availability Evidence UTC",
    "Alternate MPN / Source",
    "Cost Basis",
    "BOM Comments",
]

DEFAULT_LIFECYCLE = (
    "Lifecycle not machine-verified; check manufacturer PCN/EOL sources before release"
)


def natural_ref_key(reference: str) -> tuple[str, int, str]:
    match = re.fullmatch(r"([A-Za-z]+)(\d+)(.*)", reference)
    if not match:
        return (reference, 0, "")
    return (match.group(1), int(match.group(2)), match.group(3))


def run_kicad_xml_export(schematic: Path, xml_out: Path) -> None:
    xml_out.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "kicad-cli",
            "sch",
            "export",
            "netlist",
            "--format",
            "kicadxml",
            "--output",
            str(xml_out),
            str(schematic),
        ],
        check=True,
    )


def extract_rows(xml_path: Path) -> list[OrderedDict[str, str]]:
    tree = ET.parse(xml_path)
    root = tree.getroot()
    components = root.find("components")
    if components is None:
        raise RuntimeError(f"No <components> in {xml_path}")

    rows: list[OrderedDict[str, str]] = []
    for comp in components.findall("comp"):
        reference = (comp.get("ref") or "").strip()
        value = (comp.findtext("value") or "").strip()
        footprint = (comp.findtext("footprint") or "").strip()
        datasheet = (comp.findtext("datasheet") or "").strip()

        props: dict[str, str] = {}
        fields = comp.find("fields")
        if fields is not None:
            for field in fields.findall("field"):
                name = (field.get("name") or "").strip()
                text = (field.text or "").strip()
                if name:
                    props[name] = text
        for prop in comp.findall("property"):
            name = (prop.get("name") or "").strip()
            value_attr = (prop.get("value") or "").strip()
            if name and value_attr and name not in props:
                props[name] = value_attr

        price_date = props.get("Price Observed UTC", "")
        lifecycle = props.get("Lifecycle Status", "") or DEFAULT_LIFECYCLE
        lifecycle_date = props.get("Lifecycle/Availability Evidence UTC", "") or price_date

        row: OrderedDict[str, str] = OrderedDict()
        row["Reference"] = reference
        row["Value"] = value
        row["Manufacturer"] = props.get("Manufacturer", "")
        row["MPN"] = props.get("MPN", "")
        row["Footprint"] = footprint
        row["Datasheet"] = datasheet
        row["Supplier"] = props.get("Supplier", "")
        row["Supplier PN"] = props.get("Supplier PN", "")
        row["Estimated Unit Cost USD"] = props.get("Estimated Unit Cost USD", "")
        row["Price Observed UTC"] = price_date
        row["Stock Observed"] = props.get("Stock Observed", "")
        row["Lifecycle Status"] = lifecycle
        row["Lifecycle/Availability Evidence UTC"] = lifecycle_date
        row["Alternate MPN / Source"] = props.get("Alternate MPN / Source", "")
        row["Cost Basis"] = props.get("Cost Basis", "")
        row["BOM Comments"] = props.get("BOM Comments", "")
        rows.append(row)

    rows.sort(key=lambda row: natural_ref_key(row["Reference"]))
    return rows


def write_component_csv(rows: list[OrderedDict[str, str]], out_path: Path) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", newline="") as file_obj:
        writer = csv.DictWriter(file_obj, fieldnames=REQUIRED_COLUMNS)
        writer.writeheader()
        writer.writerows(rows)


def group_physical_bom(rows: list[OrderedDict[str, str]]) -> list[OrderedDict[str, str]]:
    physical_rows = [row for row in rows if not row["Reference"].startswith("TP")]

    group_key_fields = [
        "Value",
        "Manufacturer",
        "MPN",
        "Footprint",
        "Datasheet",
        "Supplier",
        "Supplier PN",
        "Estimated Unit Cost USD",
        "Price Observed UTC",
        "Stock Observed",
        "Lifecycle Status",
        "Lifecycle/Availability Evidence UTC",
        "Alternate MPN / Source",
        "Cost Basis",
    ]

    grouped: OrderedDict[tuple[str, ...], dict[str, list[str]]] = OrderedDict()
    for row in physical_rows:
        key = tuple(row[field] for field in group_key_fields)
        bucket = grouped.setdefault(key, {"refs": [], "comments": []})
        bucket["refs"].append(row["Reference"])
        comment = row["BOM Comments"].strip()
        if comment and comment not in bucket["comments"]:
            bucket["comments"].append(comment)

    bom_rows: list[OrderedDict[str, str]] = []
    for key, bucket in grouped.items():
        refs = sorted(bucket["refs"], key=natural_ref_key)
        row: OrderedDict[str, str] = OrderedDict()
        row["References"] = ", ".join(refs)
        row["Qty"] = str(len(refs))
        for field, value in zip(group_key_fields, key):
            row[field] = value
        row["BOM Comments"] = "; ".join(bucket["comments"])
        bom_rows.append(row)

    bom_rows.sort(key=lambda row: natural_ref_key(row["References"].split(",")[0].strip()))
    return bom_rows


def write_bom_csv(rows: list[OrderedDict[str, str]], out_path: Path) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    key_fields = [
        "Value",
        "Manufacturer",
        "MPN",
        "Footprint",
        "Datasheet",
        "Supplier",
        "Supplier PN",
        "Estimated Unit Cost USD",
        "Price Observed UTC",
        "Stock Observed",
        "Lifecycle Status",
        "Lifecycle/Availability Evidence UTC",
        "Alternate MPN / Source",
        "Cost Basis",
        "BOM Comments",
    ]
    with out_path.open("w", newline="") as file_obj:
        writer = csv.DictWriter(file_obj, fieldnames=["References", "Qty", *key_fields])
        writer.writeheader()
        writer.writerows(rows)


def write_analysis_json(component_rows: list[OrderedDict[str, str]], bom_rows: list[OrderedDict[str, str]], out_path: Path) -> None:
    missing: list[dict[str, str]] = []
    required_non_tp_fields = [
        "Manufacturer",
        "MPN",
        "Footprint",
        "Datasheet",
        "Supplier",
        "Supplier PN",
        "Estimated Unit Cost USD",
        "Price Observed UTC",
        "Lifecycle Status",
    ]
    for row in component_rows:
        if row["Reference"].startswith("TP"):
            continue
        for field in required_non_tp_fields:
            if not row.get(field, "").strip():
                missing.append({"Reference": row["Reference"], "Missing Field": field})

    analysis = {
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "component_rows_total": len(component_rows),
        "physical_bom_line_items": len(bom_rows),
        "excluded_from_physical_bom_refs": [
            row["Reference"] for row in component_rows if row["Reference"].startswith("TP")
        ],
        "missing_required_fields_non_testpoints": missing,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(analysis, indent=2) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("schematic", type=Path, help="Path to .kicad_sch schematic")
    parser.add_argument(
        "--xml-out",
        type=Path,
        default=Path("docs/verification/spool-sentry.xml"),
        help="Output XML netlist path",
    )
    parser.add_argument(
        "--component-csv",
        type=Path,
        default=Path("docs/verification/component-properties.csv"),
        help="Per-component property export CSV",
    )
    parser.add_argument(
        "--bom-csv",
        type=Path,
        default=Path("bom/bom.csv"),
        help="Grouped physical BOM CSV output",
    )
    parser.add_argument(
        "--analysis-json",
        type=Path,
        default=Path("docs/verification/bom-analysis.json"),
        help="BOM coverage summary JSON",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    run_kicad_xml_export(args.schematic, args.xml_out)
    component_rows = extract_rows(args.xml_out)
    write_component_csv(component_rows, args.component_csv)
    bom_rows = group_physical_bom(component_rows)
    write_bom_csv(bom_rows, args.bom_csv)
    write_analysis_json(component_rows, bom_rows, args.analysis_json)
    print(
        f"Exported {len(component_rows)} components and {len(bom_rows)} physical BOM lines"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
