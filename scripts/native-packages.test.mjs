import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  NATIVE_PACKAGES,
  artifactName,
  onnxArtifactName,
  stageNativePackages,
  syncNativePackages,
} from "./native-packages.mjs";

const VERSION = "7.8.9";

async function writeJson(path, value) {
  await mkdir(join(path, ".."), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

async function fixtureRoot() {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-packages-"));
  await writeJson(join(root, "package.json"), { version: VERSION });
  for (const entry of NATIVE_PACKAGES) {
    await writeJson(join(root, entry.directory, "package.json"), {
      name: entry.name,
      version: "0.0.0",
    });
  }
  for (const kind of new Set(NATIVE_PACKAGES.map(({ kind }) => kind))) {
    await writeJson(join(root, "packages", kind, "package.json"), {
      name: `@jongleberry/vurst-${kind}`,
      optionalDependencies: { stale: "0.0.0" },
    });
    await writeFile(
      join(root, "packages", kind, "index.js"),
      "if (bindingPackageVersion !== '0.2.1') throw new Error('expected 0.2.1');\n",
    );
  }
  return root;
}

test("the native package matrix covers the supported targets exactly once", () => {
  assert.equal(NATIVE_PACKAGES.length, 9);
  assert.deepEqual(
    new Set(NATIVE_PACKAGES.map(({ kind }) => kind)),
    new Set(["ai", "html", "markdown"]),
  );
  for (const kind of ["ai", "html", "markdown"]) {
    assert.deepEqual(
      NATIVE_PACKAGES.filter((entry) => entry.kind === kind).map(
        ({ platform }) => platform,
      ),
      ["darwin-arm64", "linux-arm64-gnu", "linux-x64-gnu"],
    );
  }
});

test("sync pins parent optional dependencies and generated loader checks", async () => {
  const root = await fixtureRoot();
  try {
    await syncNativePackages({ root, version: VERSION });
    for (const entry of NATIVE_PACKAGES) {
      const manifest = JSON.parse(
        await readFile(join(root, entry.directory, "package.json"), "utf8"),
      );
      assert.equal(manifest.version, VERSION);
    }
    for (const kind of ["ai", "html", "markdown"]) {
      const manifest = JSON.parse(
        await readFile(join(root, "packages", kind, "package.json"), "utf8"),
      );
      assert.deepEqual(
        manifest.optionalDependencies,
        Object.fromEntries(
          NATIVE_PACKAGES.filter((entry) => entry.kind === kind).map((entry) => [
            entry.name,
            VERSION,
          ]),
        ),
      );
      const loader = await readFile(join(root, "packages", kind, "index.js"), "utf8");
      assert.match(loader, new RegExp(`bindingPackageVersion !== '${VERSION}'`));
      assert.match(loader, new RegExp(`expected ${VERSION}`));
      assert.doesNotMatch(loader, /0\.2\.[14]/);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("sync rejects versions that cannot safely be written into a loader", async () => {
  await assert.rejects(
    syncNativePackages({ root: "/unused", version: "1.2.3'; process.exit()" }),
    /Invalid package version/,
  );
});

test("stage copies every target artifact and each AI runtime library", async () => {
  const root = await fixtureRoot();
  const artifacts = join(root, "artifacts");
  try {
    await mkdir(artifacts);
    for (const entry of NATIVE_PACKAGES) {
      await writeFile(join(artifacts, artifactName(entry, VERSION)), entry.name);
      if (entry.kind === "ai") {
        await writeFile(
          join(artifacts, onnxArtifactName(entry, VERSION)),
          `onnx ${entry.platform}`,
        );
      }
    }
    const staged = await stageNativePackages({ root, artifacts, version: VERSION });
    assert.equal(staged.length, 12);
    for (const entry of NATIVE_PACKAGES) {
      assert.equal(
        await readFile(join(root, entry.directory, entry.binary), "utf8"),
        entry.name,
      );
      if (entry.kind === "ai") {
        assert.equal(
          await readFile(
            join(root, entry.directory, "onnxruntime", `libonnxruntime${entry.extension}`),
            "utf8",
          ),
          `onnx ${entry.platform}`,
        );
      }
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("stage validates every artifact before copying any package payload", async () => {
  const root = await fixtureRoot();
  const artifacts = join(root, "artifacts");
  try {
    await mkdir(artifacts);
    for (const entry of NATIVE_PACKAGES.slice(0, -1)) {
      await writeFile(join(artifacts, artifactName(entry, VERSION)), entry.name);
      if (entry.kind === "ai") {
        await writeFile(
          join(artifacts, onnxArtifactName(entry, VERSION)),
          `onnx ${entry.platform}`,
        );
      }
    }

    await assert.rejects(
      stageNativePackages({ root, artifacts, version: VERSION }),
      /Missing release artifact/,
    );
    for (const entry of NATIVE_PACKAGES) {
      await assert.rejects(
        readFile(join(root, entry.directory, entry.binary)),
        (error) => error.code === "ENOENT",
      );
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("the checked-in package metadata and AI wrappers follow the matrix", async () => {
  const rootManifest = JSON.parse(await readFile("package.json", "utf8"));
  for (const entry of NATIVE_PACKAGES) {
    const manifest = JSON.parse(
      await readFile(join(entry.directory, "package.json"), "utf8"),
    );
    assert.equal(manifest.name, entry.name);
    assert.equal(manifest.version, rootManifest.version);
    assert.deepEqual(manifest.os, [entry.os]);
    assert.deepEqual(manifest.cpu, [entry.cpu]);
    assert.deepEqual(manifest.libc, entry.libc ? [entry.libc] : undefined);
    assert.equal(manifest.publishConfig.access, "public");
    assert.equal(manifest.publishConfig.provenance, true);
    assert.equal(manifest.repository.directory, entry.directory);
    if (entry.kind === "ai") {
      assert.equal(manifest.main, "index.js");
      assert.deepEqual(manifest.files, [
        "index.js",
        "*.node",
        "onnxruntime/",
        "README.md",
        "LICENSE",
      ]);
      const wrapper = await readFile(join(entry.directory, "index.js"), "utf8");
      assert.match(wrapper, new RegExp(`require\\(\"\\./${entry.binary.replaceAll(".", "\\.")}\"\\)`));
      assert.match(wrapper, new RegExp(`libonnxruntime\\${entry.extension}`));
      assert.match(wrapper, /if \(!process\.env\.ORT_DYLIB_PATH\)/);
    } else {
      assert.equal(manifest.main, entry.binary);
      assert.deepEqual(manifest.files, ["*.node", "README.md", "LICENSE"]);
    }
    await readFile(join(entry.directory, "README.md"), "utf8");
    await readFile(join(entry.directory, "LICENSE"), "utf8");
  }
});
