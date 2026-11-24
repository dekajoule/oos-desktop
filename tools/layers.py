#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""The direction of the parts (ADR 0003): each workspace crate may depend only on the crates listed for it."""
import json
import subprocess
import sys

ALLOWED = {
    "oos-link": set(),
    "oos-scpi": {"oos-link"},
    "oos-session": {"oos-scpi", "oos-link"},
    "oos-desktop": {"oos-session"},
}


def main():
    meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"]))
    ours = {p["name"] for p in meta["packages"]}
    bad = []
    for p in meta["packages"]:
        if p["name"] not in ALLOWED:
            bad.append(f"{p['name']}: a crate tools/layers.py does not know; give it its place in ALLOWED and ADR 0003")
            continue
        for d in p["dependencies"]:
            if d["name"] in ours and d["name"] not in ALLOWED[p["name"]]:
                bad.append(f"{p['name']} -> {d['name']}: against the direction of ADR 0003")
    for b in bad:
        print(b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
