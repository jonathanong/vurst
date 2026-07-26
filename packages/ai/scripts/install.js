#!/usr/bin/env node
"use strict";

const { createHash, randomBytes } = require("node:crypto");
const { createReadStream, createWriteStream, existsSync } = require("node:fs");
const {
  copyFile,
  mkdir,
  readFile,
  rename,
  rm,
  writeFile,
} = require("node:fs/promises");
const http = require("node:http");
const https = require("node:https");
const { basename, dirname, join } = require("node:path");
const { pipeline } = require("node:stream/promises");
const { fileURLToPath } = require("node:url");

const REPOSITORY = "jonathanong/vurst";
const DOWNLOAD_TIMEOUT_MS = 30_000;
const MAX_REDIRECTS = 5;
const MAX_ATTEMPTS = 6;
const RETRY_BASE_MS = 1_000;
const RETRYABLE_CODES = new Set([
  "EAI_AGAIN",
  "ECONNREFUSED",
  "ECONNRESET",
  "ENETUNREACH",
  "EPIPE",
  "ERR_STREAM_PREMATURE_CLOSE",
  "ETIMEDOUT",
]);

const TARGETS = {
  "darwin-arm64": {
    triple: "aarch64-apple-darwin",
    napiSuffix: "darwin-arm64",
    onnxDirectory: "darwin-arm64",
    onnxLibrary: "libonnxruntime.dylib",
    onnxAssetExtension: ".dylib",
  },
  "linux-x64": {
    triple: "x86_64-unknown-linux-gnu",
    napiSuffix: "linux-x64-gnu",
    onnxDirectory: "linux-x64",
    onnxLibrary: "libonnxruntime.so",
    onnxAssetExtension: ".so",
  },
  "linux-arm64": {
    triple: "aarch64-unknown-linux-gnu",
    napiSuffix: "linux-arm64-gnu",
    onnxDirectory: "linux-arm64",
    onnxLibrary: "libonnxruntime.so",
    onnxAssetExtension: ".so",
  },
};

class HttpError extends Error {
  constructor(url, statusCode) {
    super(`Download failed for ${url}: HTTP ${statusCode}`);
    this.statusCode = statusCode;
    this.retryable =
      statusCode === 408 ||
      statusCode === 429 ||
      (statusCode >= 500 && statusCode < 600);
  }
}

function detectMusl(report = process.report) {
  try {
    const current =
      typeof report?.getReport === "function" ? report.getReport() : null;
    if (current?.header?.glibcVersionRuntime) {
      return false;
    }
    if (
      current?.sharedObjects?.some(
        (path) => path.includes("libc.musl-") || path.includes("ld-musl-"),
      )
    ) {
      return true;
    }
  } catch {
    // Fall through to the ldd probe.
  }

  try {
    return require("node:child_process")
      .execFileSync("ldd", ["--version"], { encoding: "utf8" })
      .includes("musl");
  } catch {
    return false;
  }
}

function platformTarget(
  platform = process.platform,
  arch = process.arch,
  musl = detectMusl(),
) {
  if (platform === "darwin" && arch === "arm64") {
    return TARGETS["darwin-arm64"];
  }
  if (platform === "linux" && arch === "x64" && !musl) {
    return TARGETS["linux-x64"];
  }
  if (platform === "linux" && arch === "arm64" && !musl) {
    return TARGETS["linux-arm64"];
  }
  return null;
}

function packageKind(packageName) {
  const match = /^@jongleberry\/vurst-(ai|html|markdown)$/.exec(packageName);
  if (!match) {
    throw new Error(`Unsupported vurst native package: ${packageName}`);
  }
  return match[1];
}

function releaseBaseUrl(version) {
  return (
    process.env.VURST_RELEASE_BASE_URL ||
    `https://github.com/${REPOSITORY}/releases/download/v${version}`
  );
}

function normalizeBaseUrl(value) {
  const url = String(value);
  return url.endsWith("://") ? url : url.replace(/\/+$/, "");
}

function validateBaseUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Invalid release base URL: ${value}`);
  }
  if (url.username || url.password) {
    throw new Error("Release download URLs must not contain credentials");
  }
  if (url.protocol === "file:") {
    if (!String(value).toLowerCase().startsWith("file:///")) {
      throw new Error("File release URLs must use canonical file:/// form");
    }
    return url;
  }
  const local = url.hostname === "127.0.0.1" || url.hostname === "localhost";
  if (url.protocol !== "https:" && !(local && url.protocol === "http:")) {
    throw new Error("Release download URLs must use HTTPS");
  }
  return url;
}

function validateDownloadUrl(value, baseUrl) {
  const url = validateBaseUrl(value);
  if (url.protocol === "file:") {
    if (baseUrl.protocol !== "file:") {
      throw new Error(`Untrusted release redirect: ${value}`);
    }
    return;
  }
  const sameOrigin = url.origin === baseUrl.origin;
  const githubAssetHost =
    baseUrl.hostname === "github.com" &&
    (url.hostname === "githubusercontent.com" ||
      url.hostname.endsWith(".githubusercontent.com"));
  if (!sameOrigin && !githubAssetHost) {
    throw new Error(`Untrusted release redirect: ${value}`);
  }
}

function request(url, handleResponse, baseUrl, redirects = 0) {
  validateDownloadUrl(url, baseUrl);
  return new Promise((resolve, reject) => {
    const client = url.startsWith("http://") ? http : https;
    const req = client.get(url, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
        response.resume();
        if (redirects >= MAX_REDIRECTS) {
          reject(new Error(`Too many redirects while downloading ${url}`));
          return;
        }
        if (!response.headers.location) {
          reject(new Error(`Redirect missing Location header for ${url}`));
          return;
        }
        const redirected = new URL(response.headers.location, url).toString();
        request(redirected, handleResponse, baseUrl, redirects + 1).then(
          resolve,
          reject,
        );
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(new HttpError(url, response.statusCode));
        return;
      }
      Promise.resolve(handleResponse(response)).then(resolve, reject);
    });
    req.setTimeout(DOWNLOAD_TIMEOUT_MS, () => {
      const error = new Error(`Download timed out after ${DOWNLOAD_TIMEOUT_MS}ms`);
      error.retryable = true;
      req.destroy(error);
    });
    req.on("error", reject);
  });
}

function retryable(error) {
  return error?.retryable === true || RETRYABLE_CODES.has(error?.code);
}

async function withRetry(operation) {
  for (let attempt = 1; ; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      if (attempt >= MAX_ATTEMPTS || !retryable(error)) {
        throw error;
      }
      const maximum = Math.min(RETRY_BASE_MS * 2 ** (attempt - 1), 30_000);
      const delay = Math.floor(Math.random() * maximum);
      console.warn(
        `vurst: retrying release download after ${error.message} ` +
          `(attempt ${attempt + 1}/${MAX_ATTEMPTS})`,
      );
      await new Promise((resolve) => setTimeout(resolve, delay));
    }
  }
}

async function download(url, destination, baseUrl) {
  validateDownloadUrl(url, baseUrl);
  if (new URL(url).protocol === "file:") {
    await copyFile(fileURLToPath(url), destination);
    return;
  }
  await withRetry(() =>
    request(
      url,
      (response) => pipeline(response, createWriteStream(destination)),
      baseUrl,
    ),
  );
}

async function fetchText(url, baseUrl) {
  validateDownloadUrl(url, baseUrl);
  if (new URL(url).protocol === "file:") {
    return readFile(fileURLToPath(url), "utf8");
  }
  return withRetry(async () => {
    const chunks = [];
    let length = 0;
    await request(
      url,
      async (response) => {
        for await (const chunk of response) {
          length += chunk.length;
          if (length > 1024 * 1024) {
            throw new Error("Checksum response exceeded 1 MiB");
          }
          chunks.push(chunk);
        }
      },
      baseUrl,
    );
    return Buffer.concat(chunks).toString("utf8");
  });
}

function parseChecksum(text, expectedAsset) {
  for (const line of text.split(/\r?\n/)) {
    const [hash, file] = line.trim().split(/\s+/, 2);
    const normalizedFile = file?.replace(/^\*/, "");
    if (
      /^[a-fA-F0-9]{64}$/.test(hash || "") &&
      (!file ||
        normalizedFile === expectedAsset ||
        basename(normalizedFile) === expectedAsset)
    ) {
      return hash.toLowerCase();
    }
  }
  throw new Error(`No SHA-256 checksum found for ${expectedAsset}`);
}

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

function requiredAssets(kind, version, target, packageRoot) {
  const binaryName = `vurst-${kind}`;
  const assets = [
    {
      name: `${binaryName}-v${version}-${target.triple}.node`,
      destination: join(packageRoot, `${binaryName}.${target.napiSuffix}.node`),
    },
  ];
  if (kind === "ai") {
    assets.push({
      name:
        `vurst-ai-onnxruntime-v${version}-${target.triple}` +
        target.onnxAssetExtension,
      destination: join(
        packageRoot,
        "onnxruntime",
        target.onnxDirectory,
        target.onnxLibrary,
      ),
    });
  }
  return assets;
}

async function installedVersion(marker) {
  try {
    return (await readFile(marker, "utf8")).trim();
  } catch (error) {
    if (error.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function installNative(options = {}) {
  const packageRoot = options.packageRoot || join(__dirname, "..");
  const pkg = JSON.parse(await readFile(join(packageRoot, "package.json"), "utf8"));
  const kind = packageKind(pkg.name);
  const target =
    options.target ||
    platformTarget(options.platform, options.arch, options.musl);
  if (!target) {
    throw new Error(
      `Unsupported platform ${options.platform || process.platform}/` +
        `${options.arch || process.arch}; vurst provides macOS arm64 and ` +
        "Linux x64/arm64 glibc binaries",
    );
  }
  if (process.env.VURST_SKIP_BINARY_DOWNLOAD === "1") {
    console.log(`Skipping ${pkg.name} native download`);
    return [];
  }

  const marker = join(packageRoot, ".vurst-native-version");
  const assets = requiredAssets(kind, pkg.version, target, packageRoot);
  if (
    (await installedVersion(marker)) === pkg.version &&
    assets.every(({ destination }) => existsSync(destination))
  ) {
    console.log(`${pkg.name} native assets already match v${pkg.version}`);
    return assets.map(({ destination }) => destination);
  }

  const base = normalizeBaseUrl(options.baseUrl || releaseBaseUrl(pkg.version));
  const parsedBase = validateBaseUrl(base);
  const temporary = [];
  try {
    for (const asset of assets) {
      await mkdir(dirname(asset.destination), { recursive: true });
      const temp = `${asset.destination}.tmp-${randomBytes(8).toString("hex")}`;
      temporary.push(temp);
      console.log(`Downloading ${asset.name}`);
      await download(`${base}/${asset.name}`, temp, parsedBase);
      const checksumText = await fetchText(
        `${base}/${asset.name}.sha256`,
        parsedBase,
      );
      const expected = parseChecksum(checksumText, asset.name);
      const actual = await sha256(temp);
      if (actual !== expected) {
        throw new Error(
          `Checksum mismatch for ${asset.name}: expected ${expected}, got ${actual}`,
        );
      }
    }

    for (let index = 0; index < assets.length; index += 1) {
      await rename(temporary[index], assets[index].destination);
    }
    const markerTemp = `${marker}.tmp-${randomBytes(8).toString("hex")}`;
    temporary.push(markerTemp);
    await writeFile(markerTemp, `${pkg.version}\n`);
    await rename(markerTemp, marker);
    return assets.map(({ destination }) => destination);
  } catch (error) {
    await Promise.all(temporary.map((path) => rm(path, { force: true })));
    throw new Error(`Failed to install ${pkg.name} native assets: ${error.message}`);
  }
}

async function main() {
  try {
    await installNative();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

if (require.main === module) {
  void main();
}

module.exports = {
  TARGETS,
  installNative,
  installedVersion,
  packageKind,
  parseChecksum,
  platformTarget,
  requiredAssets,
  sha256,
  validateBaseUrl,
};
