/* global process */

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";

const root = process.cwd();
const ignored = new Set([".git", "dist", "node_modules"]);

function markdownFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (ignored.has(entry.name)) return [];
    const absolute = resolve(directory, entry.name);
    if (entry.isDirectory()) return markdownFiles(absolute);
    return extname(entry.name) === ".md" ? [absolute] : [];
  });
}

const failures = [];
for (const file of markdownFiles(root)) {
  const text = readFileSync(file, "utf8");
  for (const match of text.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
    const raw = match[1];
    if (!raw || raw.startsWith("#") || /^[a-z][a-z0-9+.-]*:/i.test(raw))
      continue;
    const clean = decodeURIComponent(raw.split("#")[0].replace(/^<|>$/g, ""));
    if (clean.length === 0) continue;
    const target = resolve(dirname(file), clean);
    if (
      !existsSync(target) ||
      (existsSync(target) && statSync(target).isDirectory())
    ) {
      failures.push(`${file.slice(root.length + 1)} -> ${raw}`);
    }
  }
}

if (failures.length > 0) {
  process.stderr.write(
    `Broken local Markdown links:\n${failures.join("\n")}\n`,
  );
  process.exitCode = 1;
}
