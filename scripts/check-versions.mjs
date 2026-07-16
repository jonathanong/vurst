import { readFile } from "node:fs/promises";

const npmPackagePaths = [
  "package.json",
  "packages/ai/package.json",
  "packages/html/package.json",
  "packages/markdown/package.json",
  "packages/prompt/package.json",
  "packages/runtime/package.json",
];

const cargoPackagePaths = [
  "packages/ai/Cargo.toml",
  "packages/html/Cargo.toml",
  "packages/markdown/Cargo.toml",
];

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

async function readCargoVersion(path) {
  const cargo = await readFile(path, "utf8");
  const match = cargo.match(/^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error(`${path} does not contain a package version`);
  }

  return match[1];
}

const entries = [];

for (const path of npmPackagePaths) {
  const pkg = await readJson(path);
  entries.push([path, pkg.version]);
}

for (const path of cargoPackagePaths) {
  entries.push([path, await readCargoVersion(path)]);
}

const expected = entries[0]?.[1];
const mismatches = entries.filter(([, version]) => version !== expected);

if (mismatches.length > 0) {
  console.error(`Expected all package versions to be ${expected}.`);
  for (const [path, version] of entries) {
    console.error(`${path}: ${version}`);
  }
  process.exit(1);
}

console.log(`All package versions are ${expected}.`);
