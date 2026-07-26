import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { pathToFileURL } from "node:url";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const installer = require("../packages/ai/scripts/install.js");
const packageNames = {
  ai: "@jongleberry/vurst-ai",
  html: "@jongleberry/vurst-html",
  markdown: "@jongleberry/vurst-markdown",
};
const version = "9.8.7";
const target = installer.TARGETS["linux-x64"];

async function createPackage(root, kind) {
  const packageRoot = join(root, kind);
  await mkdir(packageRoot, { recursive: true });
  await writeFile(
    join(packageRoot, "package.json"),
    `${JSON.stringify({ name: packageNames[kind], version }, null, 2)}\n`,
  );
  return packageRoot;
}

async function createReleaseAssets(releaseRoot, kind, packageRoot, badAsset) {
  await mkdir(releaseRoot, { recursive: true });
  const assets = installer.requiredAssets(kind, version, target, packageRoot);
  for (const asset of assets) {
    const content = Buffer.from(`release bytes for ${asset.name}\n`);
    const checksumContent =
      asset.name === badAsset ? Buffer.from("different bytes") : content;
    const hash = createHash("sha256").update(checksumContent).digest("hex");
    await writeFile(join(releaseRoot, asset.name), content);
    await writeFile(
      join(releaseRoot, `${asset.name}.sha256`),
      `${hash}  ${asset.name}\n`,
    );
  }
  return assets;
}

function releaseUrl(path) {
  return pathToFileURL(path).href;
}

test("published native packages contain identical self-contained installers", async () => {
  const paths = [
    "packages/ai/scripts/install.js",
    "packages/html/scripts/install.js",
    "packages/markdown/scripts/install.js",
  ];
  const contents = await Promise.all(paths.map((path) => readFile(path, "utf8")));
  assert.equal(contents[1], contents[0]);
  assert.equal(contents[2], contents[0]);
  assert.doesNotMatch(contents[0], /require\(["']\.\.\/\.\.\/(?:ai|html|markdown)/);
});

test("maps only the supported release targets", () => {
  assert.equal(
    installer.platformTarget("darwin", "arm64", false),
    installer.TARGETS["darwin-arm64"],
  );
  assert.equal(
    installer.platformTarget("linux", "x64", false),
    installer.TARGETS["linux-x64"],
  );
  assert.equal(
    installer.platformTarget("linux", "arm64", false),
    installer.TARGETS["linux-arm64"],
  );
  assert.equal(installer.platformTarget("linux", "x64", true), null);
  assert.equal(installer.platformTarget("darwin", "x64", false), null);
  assert.equal(installer.platformTarget("win32", "x64", false), null);
});

test("uses versioned GitHub Release names and loader destinations", () => {
  const html = installer.requiredAssets("html", version, target, "/package");
  assert.deepEqual(html, [
    {
      name: `vurst-html-v${version}-x86_64-unknown-linux-gnu.node`,
      destination: "/package/vurst-html.linux-x64-gnu.node",
    },
  ]);

  const ai = installer.requiredAssets("ai", version, target, "/package");
  assert.equal(ai.length, 2);
  assert.equal(
    ai[1].name,
    `vurst-ai-onnxruntime-v${version}-x86_64-unknown-linux-gnu.so`,
  );
  assert.equal(
    ai[1].destination,
    "/package/onnxruntime/linux-x64/libonnxruntime.so",
  );
});

test("skips only when every native asset has the current version marker", async () => {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-current-"));
  try {
    const packageRoot = await createPackage(root, "ai");
    const assets = installer.requiredAssets("ai", version, target, packageRoot);
    for (const { destination } of assets) {
      await mkdir(join(destination, ".."), { recursive: true });
      await writeFile(destination, "already installed");
    }
    await writeFile(join(packageRoot, ".vurst-native-version"), `${version}\n`);

    const installed = await installer.installNative({
      packageRoot,
      target,
      baseUrl: "https://example.invalid/should-not-be-used",
    });
    assert.deepEqual(
      installed,
      assets.map(({ destination }) => destination),
    );
    assert.equal(await readFile(assets[0].destination, "utf8"), "already installed");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

for (const markerVersion of [null, "1.0.0"]) {
  test(
    `redownloads a binary when its version marker is ${
      markerVersion === null ? "missing" : "stale"
    }`,
    async () => {
      const root = await mkdtemp(join(tmpdir(), "vurst-native-stale-"));
      try {
        const packageRoot = await createPackage(root, "html");
        const releaseRoot = join(root, "release");
        const assets = await createReleaseAssets(
          releaseRoot,
          "html",
          packageRoot,
        );
        await writeFile(assets[0].destination, "old native binary");
        if (markerVersion !== null) {
          await writeFile(
            join(packageRoot, ".vurst-native-version"),
            `${markerVersion}\n`,
          );
        }

        await installer.installNative({
          packageRoot,
          target,
          baseUrl: releaseUrl(releaseRoot),
        });
        assert.equal(
          await readFile(assets[0].destination, "utf8"),
          `release bytes for ${assets[0].name}\n`,
        );
        assert.equal(
          await readFile(join(packageRoot, ".vurst-native-version"), "utf8"),
          `${version}\n`,
        );
      } finally {
        await rm(root, { recursive: true, force: true });
      }
    },
  );
}

test("redownloads all AI assets when one current-version asset is missing", async () => {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-partial-"));
  try {
    const packageRoot = await createPackage(root, "ai");
    const releaseRoot = join(root, "release");
    const assets = await createReleaseAssets(releaseRoot, "ai", packageRoot);
    await writeFile(assets[0].destination, "old native binary");
    await writeFile(join(packageRoot, ".vurst-native-version"), `${version}\n`);

    await installer.installNative({
      packageRoot,
      target,
      baseUrl: releaseUrl(releaseRoot),
    });
    for (const asset of assets) {
      assert.equal(
        await readFile(asset.destination, "utf8"),
        `release bytes for ${asset.name}\n`,
      );
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects checksum mismatches without updating the version marker", async () => {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-checksum-"));
  try {
    const packageRoot = await createPackage(root, "markdown");
    const releaseRoot = join(root, "release");
    const assetName = `vurst-markdown-v${version}-${target.triple}.node`;
    await createReleaseAssets(
      releaseRoot,
      "markdown",
      packageRoot,
      assetName,
    );

    await assert.rejects(
      installer.installNative({
        packageRoot,
        target,
        baseUrl: releaseUrl(releaseRoot),
      }),
      /Checksum mismatch/,
    );
    await assert.rejects(
      readFile(join(packageRoot, ".vurst-native-version")),
      /ENOENT/,
    );
    const remaining = await readdir(packageRoot);
    assert.equal(
      remaining.some((name) => basename(name).includes(".tmp-")),
      false,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects unsafe release URLs", () => {
  assert.throws(
    () => installer.validateBaseUrl("http://example.com/release"),
    /must use HTTPS/,
  );
  assert.throws(
    () => installer.validateBaseUrl("https://user:secret@example.com/release"),
    /must not contain credentials/,
  );
  assert.doesNotThrow(() =>
    installer.validateBaseUrl(
      "https://github.com/jonathanong/vurst/releases/download/v9.8.7",
    ),
  );
});

test("uses standard npm and HTTPS proxy settings with NO_PROXY support", () => {
  const url =
    "https://github.com/jonathanong/vurst/releases/download/v9.8.7/asset.node";
  assert.equal(
    installer.proxyUrlFor(url, {
      HTTPS_PROXY: "http://proxy.example:8080",
    }),
    "http://proxy.example:8080",
  );
  assert.equal(
    installer.proxyUrlFor(url, {
      HTTPS_PROXY: "http://fallback.example:8080",
      npm_config_https_proxy: "http://npm-proxy.example:8080",
    }),
    "http://npm-proxy.example:8080",
  );
  assert.equal(
    installer.proxyUrlFor(url, {
      HTTPS_PROXY: "http://proxy.example:8080",
      NO_PROXY: ".github.com",
    }),
    null,
  );
  assert.equal(
    installer.proxyUrlFor("file:///tmp/release/asset.node", {
      HTTPS_PROXY: "http://proxy.example:8080",
    }),
    null,
  );
});

test("allows source builds to skip release downloads explicitly", async () => {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-skip-"));
  const previous = process.env.VURST_SKIP_BINARY_DOWNLOAD;
  try {
    const packageRoot = await createPackage(root, "markdown");
    process.env.VURST_SKIP_BINARY_DOWNLOAD = "1";
    assert.deepEqual(
      await installer.installNative({
        packageRoot,
        platform: "win32",
        arch: "x64",
        baseUrl: "https://example.invalid/should-not-be-used",
      }),
      [],
    );
  } finally {
    if (previous === undefined) {
      delete process.env.VURST_SKIP_BINARY_DOWNLOAD;
    } else {
      process.env.VURST_SKIP_BINARY_DOWNLOAD = previous;
    }
    await rm(root, { recursive: true, force: true });
  }
});
