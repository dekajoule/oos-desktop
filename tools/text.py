#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Files of the tree: LF, UTF-8, a final newline, no trailing spaces, lines of at most 120 columns in code, an SPDX
line in every source file, and every file AGENTS.md and docs/architecture.md name in backticks exists."""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = {".rs", ".py", ".sh", ".toml"}


def main():
    files = subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard"], cwd=ROOT, text=True).split()
    bad = []
    for name in files:
        p = ROOT / name
        if not p.is_file() or name == "LICENSE" or name.endswith(".lock"):
            continue
        raw = p.read_bytes()
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            continue
        if b"\r\n" in raw:
            bad.append(f"{name}: CRLF line ends")
        if raw and not raw.endswith(b"\n"):
            bad.append(f"{name}: no final newline")
        for i, line in enumerate(text.splitlines(), 1):
            if line != line.rstrip():
                bad.append(f"{name}:{i}: trailing spaces")
            if p.suffix in SOURCE and len(line) > 120:
                bad.append(f"{name}:{i}: longer than 120 columns")
        if p.suffix in SOURCE and "SPDX-License-Identifier: GPL-2.0-or-later" not in text[:400]:
            bad.append(f"{name}: no SPDX line")
    for doc in ("AGENTS.md", "docs/architecture.md"):
        for ref in re.findall(r"`((?:app|crates|tools|docs)/[^`\s:]+)", (ROOT / doc).read_text()):
            if not (ROOT / ref).exists():
                bad.append(f"{doc}: names {ref}, which does not exist")
    for b in bad:
        print(b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
