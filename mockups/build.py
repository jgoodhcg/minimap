"""Build the mockups: generate the fake repo data, then inject it into the templates.

Usage:
  python3 mockups/build.py
      Fake data -> mockups/minimap-v3.html, minimap-v4.html, and minimap-v5.html (open them in a browser).
  python3 mockups/build.py --data <index.json> --out <page.html>
      The v4 mockup from other data, such as a `minimap index` dump.
      Keep real-repo input and output under .private/.
"""
import argparse
import pathlib
import subprocess
import sys

here = pathlib.Path(__file__).parent


def page(template, data_path):
    data = pathlib.Path(data_path).read_text().replace("</", "<\\/")
    body = (here / template).read_text().replace("__DATA__", data)
    return ('<!doctype html><html><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width, initial-scale=1">'
            '<style>[hidden]{display:none!important}</style></head><body>' + body + '</body></html>')


ap = argparse.ArgumentParser()
ap.add_argument("--data", help="index JSON for the v4 mockup")
ap.add_argument("--out", help="output HTML for the v4 mockup")
args = ap.parse_args()

if args.data or args.out:
    if not (args.data and args.out):
        sys.exit("--data and --out go together")
    pathlib.Path(args.out).write_text(page("v4-template.html", args.data))
    print("wrote", args.out)
else:
    subprocess.run([sys.executable, str(here / "fakegen.py"), str(here / "fake.json")], check=True)
    for version in ("v3", "v4", "v5"):
        out = here / f"minimap-{version}.html"
        out.write_text(page(f"{version}-template.html", here / "fake.json"))
        print("wrote", out)
