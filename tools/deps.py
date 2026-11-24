#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Every direct dependency from crates.io is listed with its reason in tools/deps-allowed.txt, and nothing else is."""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent


def main():
    allowed = {}
    for line in (HERE / "deps-allowed.txt").read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            name, _, why = line.partition(" ")
            allowed[name] = why.strip()
    meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"]))
    used = {}
    for p in meta["packages"]:
        for d in p["dependencies"]:
            if d.get("path") is None:
                used.setdefault(d["name"], []).append(p["name"])
    bad = [f"{n} (used by {', '.join(c)}): not in tools/deps-allowed.txt" for n, c in used.items() if n not in allowed]
    bad += [f"{n}: in tools/deps-allowed.txt but used by no crate" for n in allowed if n not in used]
    bad += [f"{n}: no reason in tools/deps-allowed.txt" for n, why in allowed.items() if not why]
    for b in bad:
        print(b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
