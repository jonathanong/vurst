import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const packageDirectories = [
  "packages/ai",
  "packages/html",
  "packages/markdown",
  "packages/prompt",
  "packages/runtime",
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

    const { stdout: packedFileListing } = await execFileAsync("tar", [
      "-tf",
      filename,
    ]);
    const packedFiles = packedFileListing.split("\n").filter(Boolean);

    const nativeBinaries = packedFiles.filter((file) =>
      file.endsWith(".node"),
    );
    if (nativeBinaries.length > 0) {
      throw new Error(
        `${packedManifest.name} packs native binaries, expected only per-platform packages to: ${nativeBinaries.join(", ")}`,
      );
    }

    if (packedManifest.name === "@jongleberry/vurst-ai") {
      const onnxAssets = packedFiles.filter((file) =>
        file.startsWith("package/onnxruntime/"),
      );
      if (onnxAssets.length > 0) {
        throw new Error(
          `${packedManifest.name} packs onnxruntime assets, expected only per-platform packages to: ${onnxAssets.join(", ")}`,
        );
      }
    }

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
  }
} finally {
  await rm(packDirectory, { recursive: true, force: true });
}

console.log(
  `All packed manifests use registry dependency ranges for ${rootPackage.version}.`,
);
