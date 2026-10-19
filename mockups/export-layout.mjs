// Export a mockup layout as golden fixtures for the Rust port (roadmap units 003 and 019).
//
// Regenerate the committed quayside fixtures:
//
//   python3 mockups/fakegen.py mockups/fake.json
//   node mockups/export-layout.mjs
//   node mockups/export-layout.mjs --config chars
//
// Usage: node mockups/export-layout.mjs [--config tiles|chars] [<input.json> [<output-dir>]]
//
// Defaults: tiles, mockups/fake.json, and crates/core/tests/fixtures.
// The script evaluates the mockup's own layout code, the script text from
// "/* ---------- data ---------- */" to "/* ---------- colors ---------- */",
// with DATA from the input and a stub $ that returns {}.
// The data section is included because the tree code reads its `files` and `N`.
//   tiles: mockups/v3-template.html, `lay()` (the v3 tile map).
//   chars: mockups/v5-template.html, `makeLayout(CHARS, 1.0)` (the v5 one-character map).
//
// Output, with <name> = the input file name without "-files.json", or DATA.root otherwise:
//   <output-dir>/<name>-files.json         {"root", "files": [{"p", "n", "b"?, "ig"?}]} in input order.
//                                          Not written when the input is itself a -files.json.
//   <output-dir>/<name>-layout.json        tiles: {"w", "h", "tiles": [[x, y], ...], "headers": [{"path", "x", "y", "w", "h", "depth"}, ...]}
//   <output-dir>/<name>-chars-layout.json  chars: the same shape.
//
// For a real target repo, keep input and output under .private/:
//   node mockups/export-layout.mjs .private/fixtures/target-files.json .private/fixtures
//   node mockups/export-layout.mjs --config chars .private/fixtures/target-files.json .private/fixtures

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const argv = process.argv.slice(2);
const ci = argv.indexOf('--config');
const config = ci >= 0 ? argv.splice(ci, 2)[1] : 'tiles';
if (config !== 'tiles' && config !== 'chars') throw new Error(`unknown --config ${config}`);
const input = argv[0] || join(here, 'fake.json');
const outDir = argv[1] || join(here, '..', 'crates', 'core', 'tests', 'fixtures');

const template = config === 'chars' ? 'v5-template.html' : 'v3-template.html';
const html = readFileSync(join(here, template), 'utf8');
const START = '/* ---------- data ---------- */', END = '/* ---------- colors ---------- */';
const a = html.indexOf(START), b = html.indexOf(END);
if (a < 0 || b < a) throw new Error(`layout markers not found in ${template}`);
const code = html.slice(a, b);

const DATA = JSON.parse(readFileSync(input, 'utf8'));
const stub = () => ({});
const body = config === 'chars' ? 'return makeLayout(CHARS, 1.0);' : 'return lay();';
const L = new Function('DATA', '$', `${code}\n${body}`)(DATA, stub);

const fromFiles = basename(input).endsWith('-files.json');
const name = fromFiles ? basename(input).slice(0, -'-files.json'.length) : DATA.root;
mkdirSync(outDir, { recursive: true });

// One file per line keeps the fixtures diffable.
const lines = (head, key, items, tail) => `${head}\n"${key}": [\n${items.join(',\n')}\n]${tail}\n`;

if (!fromFiles) {
  const files = DATA.files.map(f => {
    const o = { p: f.p, n: f.n || 0 };
    if (f.b) o.b = 1;
    if (f.ig) o.ig = 1;
    return JSON.stringify(o);
  });
  writeFileSync(join(outDir, `${name}-files.json`), lines(`{"root": ${JSON.stringify(DATA.root)},`, 'files', files, '}'));
}

const tiles = Array.from(L.tx, (x, i) => JSON.stringify([x, L.ty[i]]));
const headers = L.headers.map(h => JSON.stringify({ path: h.node.path, x: h.x, y: h.y, w: h.w, h: h.h, depth: h.depth }));
const suffix = config === 'chars' ? '-chars-layout.json' : '-layout.json';
writeFileSync(
  join(outDir, `${name}${suffix}`),
  lines(`{"w": ${L.w}, "h": ${L.h},`, 'tiles', tiles, `,\n"headers": [\n${headers.join(',\n')}\n]}`),
);
console.log(`${name} (${config}): ${DATA.files.length} files, ${L.headers.length} headers, ${L.w} x ${L.h} cells -> ${outDir}`);
