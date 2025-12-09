#!/usr/bin/env python3
"""Compute a Cargo.lock SHA-256 hash and emit it to stdout/GITHUB_OUTPUT.

Usage: python compute_lockfile_hash.py [path_to_lockfile]
Defaults to "Cargo.lock" in the current working directory.
"""

from __future__ import annotations

import hashlib
import os
import sys
from pathlib import Path


def main() -> int:
    """Hash the Cargo.lock (or provided path) and emit it to stdout/GITHUB_OUTPUT."""
    lockfile = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("Cargo.lock")
    if not lockfile.is_file():
        print(f"Lockfile not found: {lockfile}", file=sys.stderr)
        return 1

    hash_value = hashlib.sha256(lockfile.read_bytes()).hexdigest()

    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        try:
            with open(github_output, "a", encoding="utf-8") as fh:
                fh.write(f"hash={hash_value}\n")
        except OSError as exc:
            print(f"Failed to write to GITHUB_OUTPUT: {exc}", file=sys.stderr)
            return 1

    print(f"hash={hash_value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
