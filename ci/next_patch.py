#!/usr/bin/env python3
"""
Print the next patch version from a semver string without modifying files.

Usage:
    python next_patch.py <current_version>
"""

from __future__ import annotations

import sys

from bump_version import bump_patch


def main() -> int:
    """Emit the next patch version or a non-zero status on error."""
    if len(sys.argv) != 2:
        print("usage: next_patch.py <current_version>", file=sys.stderr)
        return 1

    try:
        print(bump_patch(sys.argv[1]))
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
