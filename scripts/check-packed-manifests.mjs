import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

import { NATIVE_PACKAGES } from "./native-packages.mjs";

const execFileAsync = promisify(execFile);
const packageDirectories = [
  "packages/ai",
  "packages/html",
  "packages/markdown",
  "packages/prompt",
  "packages/runtime",
  ...NATIVE_PACKAGES.map(({ directory }) => directory),
];
const dependencyFields = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
];

const rootPackage = JSON.parse(await readFile("package.json", "utf8"));
const packDirectory = await mkdtemp(join(tmpdir(), "vurst-packed-manifests-"));

try {
  for (const packageDirectory of packageDirectories) {
    const { stdout } = await execFileAsync(
      "pnpm",
      [
        "--dir",
        packageDirectory,
        "pack",
        "--json",
        "--pack-destination",
        packDirectory,
      ],
      { maxBuffer: 10 * 1024 * 1024 },
    );
    const { filename } = JSON.parse(stdout);
    const { stdout: packedManifestJson } = await execFileAsync("tar", [
      "-xOf",
      filename,
      "package/package.json",
    ]);
    const packedManifest = JSON.parse(packedManifestJson);
    const { stdout: packedEntries } = await execFileAsync("tar", [
      "-tzf",
      filename,
    ]);

    for (const dependencyField of dependencyFields) {
      for (const [dependency, specifier] of Object.entries(
        packedManifest[dependencyField] ?? {},
      )) {
        if (specifier.startsWith("workspace:")) {
          throw new Error(
            `${packedManifest.name} packs ${dependencyField}.${dependency} as ${specifier}`,
          );
        }
      }
    }

    if (packedManifest.name === "@jongleberry/vurst-prompt") {
      const expectedHtmlRange = `^${rootPackage.version}`;
      const actualHtmlRange =
        packedManifest.dependencies?.["@jongleberry/vurst-html"];
      if (actualHtmlRange !== expectedHtmlRange) {
        throw new Error(
          `${packedManifest.name} must pack @jongleberry/vurst-html as ${expectedHtmlRange}, got ${actualHtmlRange}`,
        );
      }
    }

    if (
      [
        "@jongleberry/vurst-ai",
        "@jongleberry/vurst-html",
        "@jongleberry/vurst-markdown",
      ].includes(packedManifest.name)
    ) {
      if (packedManifest.scripts?.postinstall) {
        throw new Error(
          `${packedManifest.name} must not run an install-time binary downloader`,
        );
      }
      if (packedEntries.split("\n").some((entry) => entry.includes("scripts/install"))) {
        throw new Error(`${packedManifest.name} packs an obsolete native installer`);
      }
      if (/(^|\/)[^/]+\.node$/m.test(packedEntries)) {
        throw new Error(`${packedManifest.name} packs a native .node binary`);
      }
      if (/(^|\/)onnxruntime\//m.test(packedEntries)) {
        throw new Error(`${packedManifest.name} packs ONNX Runtime assets`);
      }

      const expectedOptionalDependencies = Object.fromEntries(
        NATIVE_PACKAGES.filter(({ kind }) => packedManifest.name.endsWith(`vurst-${kind}`)).map(
          ({ name }) => [name, rootPackage.version],
        ),
      );
      if (
        JSON.stringify(packedManifest.optionalDependencies) !==
        JSON.stringify(expectedOptionalDependencies)
      ) {
        throw new Error(
          `${packedManifest.name} must pack exact platform optional dependencies`,
        );
      }
    }

    const nativePackage = NATIVE_PACKAGES.find(({ name }) => name === packedManifest.name);
    if (nativePackage) {
      if (packedManifest.version !== rootPackage.version) {
        throw new Error(`${packedManifest.name} must pack version ${rootPackage.version}`);
      }
      if (packedManifest.publishConfig?.access !== "public") {
        throw new Error(`${packedManifest.name} must publish publicly`);
      }
      for (const required of ["package/README.md", "package/LICENSE"]) {
        if (!packedEntries.split("\n").includes(required)) {
          throw new Error(`${packedManifest.name} does not pack ${required}`);
        }
      }
    }

    if (packedManifest.name === "@jongleberry/vurst-markdown") {
      for (const entry of [
        "package/streaming-buffer.js",
        "package/streaming-buffer.d.ts",
      ]) {
        if (!packedEntries.split("\n").includes(entry)) {
          throw new Error(`${packedManifest.name} does not pack ${entry}`);
        }
      }
    }
  }
} finally {
  await rm(packDirectory, { recursive: true, force: true });
}

console.log(
  `All packed manifests use registry dependency ranges for ${rootPackage.version}.`,
);
