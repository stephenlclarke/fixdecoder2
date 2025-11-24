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
from typing import Any, Dict, List


SEVERITY_MAP: Dict[str, str] = {
    "critical": "CRITICAL",
    "high": "MAJOR",
    "medium": "MAJOR",
    "low": "MINOR",
    "informational": "INFO",
}


def load_input(path: Path) -> Dict[str, Any]:
    """Load and parse a JSON file into a dictionary."""
    if not path.exists() or path.stat().st_size == 0:
        return {"vulnerabilities": {"list": []}}
    try:
        with path.open(encoding="utf-8") as fh:
            return json.load(fh)
    except json.JSONDecodeError:
        return {"vulnerabilities": {"list": []}}


def map_issue(vuln: Dict[str, Any]) -> Dict[str, Any]:
    """Map a RustSec vulnerability entry into a Sonar generic issue."""
    advisory: Dict[str, Any] = vuln.get("advisory", {})
    package: Dict[str, Any] = vuln.get("package", {})

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
        # Generic Issue Data (current format)
        "engineId": "cargo-audit",
        "ruleId": advisory_id,
        "ruleRepository": "cargo-audit",
        "severity": sonar_severity,
        "type": "VULNERABILITY",
        "primaryLocation": {
            "message": message,
            "filePath": "Cargo.lock",
            "textRange": {"startLine": 1, "startColumn": 1, "endLine": 1, "endColumn": 1},
        },
    }


def convert(input_path: Path, output_path: Path) -> None:
    """Convert a cargo-audit JSON report into Sonar generic issues JSON."""
    data = load_input(input_path)
    vulns: List[Dict[str, Any]] = data.get("vulnerabilities", {}).get("list", [])
    issues: List[Dict[str, Any]] = [map_issue(v) for v in vulns]
    output = {"issues": issues}
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2), encoding="utf-8")


def main() -> int:
    """Entrypoint: convert input/output paths from argv into a Sonar issues file."""
    if len(sys.argv) != 3:
        print("usage: convert_rustsec_to_sonar.py <input.json> <output.json>", file=sys.stderr)
        return 1

    inp = Path(sys.argv[1])
    out = Path(sys.argv[2])

    try:
        convert(inp, out)
    except (OSError, ValueError) as exc:
        print(f"failed to convert RustSec report: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
