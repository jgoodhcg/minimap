"""Golden fuzzy-match data from telescope-fzf-native's compiled C library, for the minimap_core::fuzzy port (unit 025).

Usage:
  python3 scripts/fzf-golden.py <index-files.json> <out.json> [--lib <libfzf.so>]

The input is a `minimap index` dump or a -files.json fixture: {"root", "files": [{"p": path, ...}]}.
The output lists, per query, every matching file as [index, score] in input order,
and the positions of the TOP best-scoring files (ties in input order) as [index, positions].
The queries are generic, plus parts of names taken from the input, so no name is written here.
Keep real-repo input and output under .private/. The quayside fixture output is committed.
"""
import argparse
import ctypes
import json
import os
import sys

TOP = 50
DEFAULT_LIB = os.path.expanduser(
    "~/.local/share/nvim/site/pack/core/opt/telescope-fzf-native.nvim/build/libfzf.so")


class Positions(ctypes.Structure):
    _fields_ = [("data", ctypes.POINTER(ctypes.c_uint32)), ("size", ctypes.c_size_t), ("cap", ctypes.c_size_t)]


def load(lib_path):
    lib = ctypes.CDLL(lib_path)
    lib.fzf_parse_pattern.restype = ctypes.c_void_p
    lib.fzf_parse_pattern.argtypes = [ctypes.c_int, ctypes.c_bool, ctypes.c_char_p, ctypes.c_bool]
    lib.fzf_free_pattern.argtypes = [ctypes.c_void_p]
    lib.fzf_make_default_slab.restype = ctypes.c_void_p
    lib.fzf_get_score.restype = ctypes.c_int32
    lib.fzf_get_score.argtypes = [ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p]
    lib.fzf_get_positions.restype = ctypes.POINTER(Positions)
    lib.fzf_get_positions.argtypes = [ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p]
    lib.fzf_free_positions.argtypes = [ctypes.POINTER(Positions)]
    return lib


def queries(paths):
    """Generic queries, then name fragments from the input: whole names, prefixes, and letters skipped."""
    out = ["core", "cljs", "bc", "x", "config", "api handler", "'test", "^s", ".clj$", "!test core",
           "Core", "db", "util", "ts$", "index"]
    names = [p.rsplit("/", 1)[-1] for p in paths]
    for k in range(0, len(names), max(1, len(names) // 4)):
        stem = names[k].split(".")[0]
        if len(stem) >= 3:
            out += [stem, stem[::2]]
    return list(dict.fromkeys(q for q in out if q.isascii()))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("input")
    ap.add_argument("out")
    ap.add_argument("--lib", default=DEFAULT_LIB)
    a = ap.parse_args()
    lib = load(a.lib)
    slab = lib.fzf_make_default_slab()
    paths = [f["p"] for f in json.load(open(a.input))["files"]]
    golden = []
    for q in queries(paths):
        pat = lib.fzf_parse_pattern(0, False, ctypes.create_string_buffer(q.encode()), True)  # CaseSmart, fuzzy
        hits = []
        for i, p in enumerate(paths):
            if not p.isascii():
                continue  # the C port reads bytes; the Rust port reads chars
            b = p.encode()
            score = lib.fzf_get_score(b, pat, slab)
            if score == 0:
                continue
            pos = lib.fzf_get_positions(b, pat, slab)
            positions = sorted(set(pos.contents.data[k] for k in range(pos.contents.size))) if pos else []
            if pos:
                lib.fzf_free_positions(pos)
            hits.append([i, score, positions])
        lib.fzf_free_pattern(pat)
        top = sorted(hits, key=lambda h: (-h[1], h[0]))[:TOP]
        golden.append({"query": q, "hits": [h[:2] for h in hits], "top": [[h[0], h[2]] for h in top]})
    with open(a.out, "w") as f:
        f.write('{"queries": [\n' + ",\n".join(json.dumps(g, separators=(",", ":")) for g in golden) + "\n]}\n")
    print(f"{len(golden)} queries, {sum(len(g['hits']) for g in golden)} hits -> {a.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
