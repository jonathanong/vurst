import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createServer, get as getHttp } from "node:http";
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

async function createHttpReleaseServer(kind, packageRoot) {
  const assets = installer.requiredAssets(kind, version, target, packageRoot);
  const responses = new Map();
  for (const asset of assets) {
    const content = Buffer.from(`HTTP release bytes for ${asset.name}\n`);
    const checksum = createHash("sha256").update(content).digest("hex");
    responses.set(`/${asset.name}`, content);
    responses.set(
      `/${asset.name}.sha256`,
      Buffer.from(`${checksum}  ${asset.name}\n`),
    );
  }
  const server = createServer((request, response) => {
    const body = responses.get(new URL(request.url, "http://localhost").pathname);
    if (!body) {
      response.writeHead(404);
      response.end();
      return;
    }
    response.writeHead(200, { "content-length": body.length });
    response.end(body);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.notEqual(address, null);
  assert.equal(typeof address, "object");
  return {
    assets,
    baseUrl: `http://127.0.0.1:${address.port}`,
    httpsBaseUrl: `https://localhost:${address.port}`,
    close: () =>
      new Promise((resolve, reject) =>
        server.close((error) => (error ? reject(error) : resolve())),
      ),
  };
}

async function completesWithin(operation, milliseconds) {
  let timeout;
  try {
    return await Promise.race([
      operation,
      new Promise((_, reject) => {
        timeout = setTimeout(
          () => reject(new Error(`Native installation exceeded ${milliseconds}ms`)),
          milliseconds,
        );
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
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

test("installs AI assets over localhost HTTPS without proxy configuration", async () => {
  const root = await mkdtemp(join(tmpdir(), "vurst-native-http-"));
  const proxyVariables = [
    "npm_config_noproxy",
    "NO_PROXY",
    "no_proxy",
    "npm_config_https_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "npm_config_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "ALL_PROXY",
    "all_proxy",
  ];
  const previousProxyVariables = new Map(
    proxyVariables.map((name) => [name, process.env[name]]),
  );
  const https = require("node:https");
  const originalHttpsGet = https.get;
  let release;
  try {
    const packageRoot = await createPackage(root, "ai");
    release = await createHttpReleaseServer("ai", packageRoot);
    https.get = function getLocalHttpsRelease(...arguments_) {
      assert.equal(arguments_.length, 2);
      const [url, onResponse] = arguments_;
      assert.equal(typeof url, "string");
      assert.match(url, /^https:\/\/localhost:/);
      assert.equal(typeof onResponse, "function");
      const localUrl = new URL(url);
      localUrl.protocol = "http:";
      localUrl.hostname = "127.0.0.1";
      localUrl.port = String(new URL(release.baseUrl).port);
      return getHttp(localUrl, onResponse);
    };
    for (const name of proxyVariables) {
      delete process.env[name];
    }

    const installed = await completesWithin(
      installer.installNative({
        packageRoot,
        target,
        baseUrl: release.httpsBaseUrl,
      }),
      1_000,
    );
    assert.deepEqual(
      installed,
      release.assets.map(({ destination }) => destination),
    );
    assert.equal(release.assets.length, 2);
    const [binary, onnxLibrary] = release.assets;
    assert.equal(
      await readFile(binary.destination, "utf8"),
      `HTTP release bytes for ${binary.name}\n`,
    );
    assert.equal(
      await readFile(onnxLibrary.destination, "utf8"),
      `HTTP release bytes for ${onnxLibrary.name}\n`,
    );
    assert.equal(
      await readFile(join(packageRoot, ".vurst-native-version"), "utf8"),
      `${version}\n`,
    );
  } finally {
    https.get = originalHttpsGet;
    for (const [name, value] of previousProxyVariables) {
      if (value === undefined) {
        delete process.env[name];
      } else {
        process.env[name] = value;
      }
    }
    await release?.close();
    await rm(root, { recursive: true, force: true });
  }
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
