#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const root = process.argv[2] ? path.resolve(process.argv[2]) : process.cwd();

// "98,121,116,101" evaluates to the forbidden storage noun. 
const forbidden = String.fromCharCode(98, 121, 116, 101);
const pattern = new RegExp(`\\b${forbidden}s?\\b`, "i");

const ignoredDirs = new Set([
  ".git",
  "target",
  "node_modules"
]);

const allowFiles = new Set([
  path.normalize("bin/check-lexicon.mjs")
]);

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    const rel = path.relative(root, full);

    if (entry.isDirectory()) {
      if (!ignoredDirs.has(entry.name)) walk(full, out);
      continue;
    }

    if (!entry.isFile()) continue;
    if (allowFiles.has(path.normalize(rel))) continue;

    out.push(full);
  }
  return out;
}

const failures = [];

for (const file of walk(root)) {
  const text = fs.readFileSync(file, "utf8");
  const lines = text.split(/\r?\n/);

  for (let i = 0; i < lines.length; i++) {
    const match = pattern.exec(lines[i]);
    if (!match) continue;

    failures.push({
      file: path.relative(root, file),
      line: i + 1,
      col: match.index + 1,
      value: match[0]
    });
  }
}

if (failures.length > 0) {
  console.error("\n❌ unibit lexicon violation\n");
  console.error("Use only 8^n, 64^n, U_{1,n}, or exact bit counts.\n");

  for (const f of failures) {
    console.error(`${f.file}:${f.line}:${f.col} -> ${JSON.stringify(f.value)}`);
  }

  process.exit(1);
}

console.log("✅ unibit lexicon check passed");