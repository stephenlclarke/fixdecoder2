#!/usr/bin/env python3
"""
Convert cargo-audit JSON output into Sonar Generic Issue format.

Usage:
  python convert_rustsec_to_sonar.py <input.json> <output.json>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


SEVERITY_MAP = {
    "critical": "CRITICAL",
    "high": "MAJOR",
    "medium": "MAJOR",
    "low": "MINOR",
    "informational": "INFO",
}


def load_input(path: Path) -> dict:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def map_issue(vuln: dict) -> dict:
    advisory = vuln.get("advisory", {})
    package = vuln.get("package", {})

    advisory_id = advisory.get("id", "UNKNOWN")
    title = advisory.get("title", "RustSec advisory")
    url = advisory.get("url", "")
    severity = advisory.get("cvss") or advisory.get("severity", "")
    severity_key = str(severity).lower()
    sonar_severity = SEVERITY_MAP.get(severity_key, "MAJOR")

    message_parts = [
        f"{advisory_id}: {title}",
        f"package: {package.get('name','unknown')} {package.get('version','')}",
    ]
    if url:
        message_parts.append(f"see: {url}")
    message = " | ".join(message_parts)

    return {
        "engineId": "cargo-audit",
        "ruleId": advisory_id,
        "severity": sonar_severity,
        "type": "VULNERABILITY",
        "primaryLocation": {
            "message": message,
            "filePath": "Cargo.lock",
        },
    }


def convert(input_path: Path, output_path: Path) -> None:
    data = load_input(input_path)
    vulns = data.get("vulnerabilities", {}).get("list", [])
    issues = [map_issue(v) for v in vulns]
    output = {"issues": issues}
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2), encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: convert_rustsec_to_sonar.py <input.json> <output.json>", file=sys.stderr)
        return 1

    inp = Path(sys.argv[1])
    out = Path(sys.argv[2])

    try:
        convert(inp, out)
    except Exception as exc:  # pragma: no cover - defensive guard
        print(f"failed to convert RustSec report: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
