#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Every SCPI header the program writes (crates/scpi/src) is listed in OOS's docs/scpi.md at the commit in OOS_REV
(ADR 0002). OOS_DIR names a checkout of OOS; without it the check says that it did not run."""
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOKEN = re.compile(r"`([:*][A-Za-z][^`\s|]*)`")


def variants(h):
    """A documented header with its optional parts [..] in and out, `<n>` for a channel, without `[?]`."""
    h = h.replace("[?]", "").rstrip("?")
    out = [""]
    for part in re.split(r"(\[[^\]]*\])", h):
        if part.startswith("["):
            out = [o + x for o in out for x in ("", part[1:-1])]
        else:
            out = [o + part for o in out]
    return {re.sub(r"<n>", "{n}", o).upper() for o in out}


def documented(text):
    known = set()
    for line in text.splitlines():
        # a table names a command in its first cell; prose names some too (`:SYSTem:ERRor?`)
        cell = line.split("|")[1] if line.startswith("|") else line
        prev = None
        for t in TOKEN.findall(cell):
            # `:WAVeform:STARt[?]` / `:STOP[?]`: a second token in a cell is relative to the first one's parent
            if prev and t.startswith(":") and t.count(":") == 1:
                t = prev.rsplit(":", 1)[0] + t
            known |= variants(t)
            prev = t
    return known


def used():
    out = {}
    for f in sorted((ROOT / "crates/scpi/src").glob("*.rs")):
        for i, line in enumerate(f.read_text().splitlines(), 1):
            for lit in re.findall(r'"([^"]*)"', line):
                for cmd in lit.split(";"):
                    m = re.match(r"([:*][A-Za-z][A-Za-z0-9:{}]*)", cmd.strip())
                    if m:
                        h = re.sub(r"\{(n|ch)\}|(?<=CHANnel)\d", "{n}", m.group(1)).upper()
                        out.setdefault(h, f"{f.relative_to(ROOT)}:{i}")
    return out


def main():
    oos = os.environ.get("OOS_DIR")
    if not oos:
        print("scpi-used: not run, OOS_DIR is not set")
        return 0
    rev = (ROOT / "OOS_REV").read_text().strip()
    doc = subprocess.check_output(["git", "-C", oos, "show", f"{rev}:docs/scpi.md"], text=True)
    known = documented(doc)
    bad = [f"{where}: {h} is not in OOS docs/scpi.md at {rev[:8]}" for h, where in used().items() if h not in known]
    for b in bad:
        print(b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
