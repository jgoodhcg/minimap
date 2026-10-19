#!/usr/bin/env python3
"""Writes crates/core/tests/fixtures/quayside-details.json from mockups/fake.json.

The fixture holds, matched by path to crates/core/tests/fixtures/quayside-files.json:
- for every file, the git ages and count (`a0`, `a1`, `c`), for the heat layers (unit 008);
- for the files in one directory, and for one short test file, the shape (`m`), outline (`o`), and namespace (`ns`) (unit 007),
  and the recent commits (`r`) with their `commits` entries (unit 008).
Run `python3 mockups/build.py` first.
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DETAIL_DIR = "components/berth/src/quayside/berth/"
# A file under 48 lines, so the panel's shape test has a short file that does not stretch to the box.
DETAIL_EXTRA = ("components/berth/test/quayside/berth/core_test.clj",)
DETAIL_FIELDS = ("n", "m", "o", "ns", "r")
GIT_FIELDS = ("a0", "a1", "c")

fake = json.loads((ROOT / "mockups/fake.json").read_text())
files = []
for f in fake["files"]:
    detail = (f["p"].startswith(DETAIL_DIR) and "/" not in f["p"][len(DETAIL_DIR):]) or f["p"] in DETAIL_EXTRA
    fields = GIT_FIELDS + (DETAIL_FIELDS if detail else ())
    files.append({"p": f["p"], **{k: f[k] for k in fields if k in f}})
used = sorted({h for f in files for h in f.get("r", [])})
commits = {h: fake["commits"][h] for h in used}

out = ROOT / "crates/core/tests/fixtures/quayside-details.json"
lines = ",\n".join(json.dumps(f, separators=(",", ":")) for f in files)
commit_lines = ",\n".join(f"{json.dumps(h)}:{json.dumps(c, separators=(',', ':'))}" for h, c in commits.items())
out.write_text('{"root": "quayside",\n"files": [\n' + lines + '\n],\n"commits": {\n' + commit_lines + "\n}}\n")
print(f"{out.relative_to(ROOT)}: {len(files)} files, {len(commits)} commits")
