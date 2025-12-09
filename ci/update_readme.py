#!/usr/bin/env python3
"""Refresh README usage block from `fixdecoder --help` output."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


def generate_usage() -> str:
    """Return the CLI usage text by invoking the built binary."""
    try:
        return subprocess.check_output(
            ["./target/release/fixdecoder", "--help"],
            text=True,
            stderr=subprocess.STDOUT,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        sys.stderr.write(f"Failed to run fixdecoder --help: {exc}\n")
        sys.exit(1)


def update_readme(root: Path, usage: str) -> None:
    """Replace the usage block in README.md with the provided text."""
    readme = root / "README.md"
    text = readme.read_text(encoding="utf-8")
    pattern = r"<!-- USAGE:START -->.*?<!-- USAGE:END -->"
    replacement = f"<!-- USAGE:START -->\n```bash\n{usage}```\n<!-- USAGE:END -->"
    new_text = re.sub(pattern, replacement, text, flags=re.S)
    if new_text != text:
        readme.write_text(new_text, encoding="utf-8")


def main() -> int:
    """Entry point to refresh the README usage block."""
    root = Path(__file__).resolve().parent.parent
    usage = generate_usage()
    update_readme(root, usage)
    return 0


if __name__ == "__main__":
    sys.exit(main())
