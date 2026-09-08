import {
  cp,
  mkdir,
  readFile,
  stat,
  writeFile,
} from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(
  fileURLToPath(new URL("..", import.meta.url)),
);
const SEMVER_SOURCE =
  "(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)" +
  "(?:-[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?" +
  "(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?";
const SEMVER_PATTERN = new RegExp(`^${SEMVER_SOURCE}$`);

export const NATIVE_PACKAGES = [
  {
    kind: "ai",
    platform: "darwin-arm64",
    target: "aarch64-apple-darwin",
    extension: ".dylib",
    os: "darwin",
    cpu: "arm64",
  },
  {
    kind: "ai",
    platform: "darwin-x64",
    target: "x86_64-apple-darwin",
    extension: ".dylib",
    os: "darwin",
    cpu: "x64",
  },
  {
    kind: "ai",
    platform: "linux-arm64-gnu",
    target: "aarch64-unknown-linux-gnu",
    extension: ".so",
    os: "linux",
    cpu: "arm64",
    libc: "glibc",
  },
  {
    kind: "ai",
    platform: "linux-x64-gnu",
    target: "x86_64-unknown-linux-gnu",
    extension: ".so",
    os: "linux",
    cpu: "x64",
    libc: "glibc",
  },
  {
    kind: "html",
    platform: "darwin-arm64",
    target: "aarch64-apple-darwin",
    os: "darwin",
    cpu: "arm64",
  },
  {
    kind: "html",
    platform: "darwin-x64",
    target: "x86_64-apple-darwin",
    os: "darwin",
    cpu: "x64",
  },
  {
    kind: "html",
    platform: "linux-arm64-gnu",
    target: "aarch64-unknown-linux-gnu",
    os: "linux",
    cpu: "arm64",
    libc: "glibc",
  },
  {
    kind: "html",
    platform: "linux-x64-gnu",
    target: "x86_64-unknown-linux-gnu",
    os: "linux",
    cpu: "x64",
    libc: "glibc",
  },
  {
    kind: "markdown",
    platform: "darwin-arm64",
    target: "aarch64-apple-darwin",
    os: "darwin",
    cpu: "arm64",
  },
  {
    kind: "markdown",
    platform: "darwin-x64",
    target: "x86_64-apple-darwin",
    os: "darwin",
    cpu: "x64",
  },
  {
    kind: "markdown",
    platform: "linux-arm64-gnu",
    target: "aarch64-unknown-linux-gnu",
    os: "linux",
    cpu: "arm64",
    libc: "glibc",
  },
  {
    kind: "markdown",
    platform: "linux-x64-gnu",
    target: "x86_64-unknown-linux-gnu",
    os: "linux",
    cpu: "x64",
    libc: "glibc",
  },
].map((entry) => ({
  ...entry,
  name: `@jongleberry/vurst-${entry.kind}-${entry.platform}`,
  directory: `packages/vurst-${entry.kind}-${entry.platform}`,
  binary: `vurst-${entry.kind}.${entry.platform}.node`,
}));

function packagePath(root, entry) {
  return join(root, entry.directory, "package.json");
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function parentPackagePath(root, kind) {
  return join(root, "packages", kind, "package.json");
}

function assertPackageVersion(version) {
  if (
    typeof version !== "string" ||
    !SEMVER_PATTERN.test(version)
  ) {
    throw new Error(`Invalid package version: ${version}`);
  }
}

export async function syncNativePackages({ root = repositoryRoot, version }) {
  assertPackageVersion(version);

  for (const entry of NATIVE_PACKAGES) {
    const path = packagePath(root, entry);
    const pkg = await readJson(path);
    pkg.version = version;
    await writeJson(path, pkg);
  }

  for (const kind of new Set(NATIVE_PACKAGES.map(({ kind }) => kind))) {
    const path = parentPackagePath(root, kind);
    const pkg = await readJson(path);
    pkg.optionalDependencies = Object.fromEntries(
      NATIVE_PACKAGES.filter((entry) => entry.kind === kind).map((entry) => [
        entry.name,
        version,
      ]),
    );
    await writeJson(path, pkg);

    const loaderPath = join(root, "packages", kind, "index.js");
    const loader = await readFile(loaderPath, "utf8");
    const synchronized = loader
      .replaceAll(
        /bindingPackageVersion !== '[^']+'/g,
        `bindingPackageVersion !== '${version}'`,
      )
      .replaceAll(
        new RegExp(`expected ${SEMVER_SOURCE}`, "g"),
        `expected ${version}`,
      );
    await writeFile(loaderPath, synchronized);
  }
}

async function requireRegularFile(path) {
  let details;
  try {
    details = await stat(path);
  } catch {
    throw new Error(`Missing release artifact: ${path}`);
  }
  if (!details.isFile() || details.size === 0) {
    throw new Error(`Release artifact must be a non-empty file: ${path}`);
  }
}

function artifactName(entry, version) {
  return `vurst-${entry.kind}-v${version}-${entry.target}.node`;
}

function onnxArtifactName(entry, version) {
  return `vurst-ai-onnxruntime-v${version}-${entry.target}${entry.extension}`;
}

export async function stageNativePackages({
  root = repositoryRoot,
  artifacts,
  version,
}) {
  if (!artifacts) throw new Error("--artifacts is required.");
  if (!version) {
    version = (await readJson(join(root, "package.json"))).version;
  }
  assertPackageVersion(version);

  const artifactDirectory = resolve(root, artifacts);
  const requiredArtifacts = NATIVE_PACKAGES.flatMap((entry) => [
    join(artifactDirectory, artifactName(entry, version)),
    ...(entry.kind === "ai"
      ? [join(artifactDirectory, onnxArtifactName(entry, version))]
      : []),
  ]);
  await Promise.all(requiredArtifacts.map(requireRegularFile));

  const staged = [];
  for (const entry of NATIVE_PACKAGES) {
    const source = join(artifactDirectory, artifactName(entry, version));
    const destination = join(root, entry.directory, entry.binary);
    await mkdir(join(root, entry.directory), { recursive: true });
    await cp(source, destination);
    staged.push(destination);

    if (entry.kind === "ai") {
      const onnxSource = join(artifactDirectory, onnxArtifactName(entry, version));
      const onnxDestination = join(
        root,
        entry.directory,
        "onnxruntime",
        `libonnxruntime${entry.extension}`,
      );
      await mkdir(join(onnxDestination, ".."), { recursive: true });
      await cp(onnxSource, onnxDestination);
      staged.push(onnxDestination);
    }
  }
  return staged;
}

function argument(argv, name) {
  const index = argv.indexOf(name);
  if (index === -1 || !argv[index + 1]) throw new Error(`${name} is required.`);
  return argv[index + 1];
}

async function main(argv = process.argv.slice(2)) {
  const [command] = argv;
  if (command === "sync") {
    const version = argv[1];
    if (!version || argv.length !== 2) {
      throw new Error("usage: native-packages.mjs sync <version>");
    }
    await syncNativePackages({ version });
    return;
  }
  if (command === "stage") {
    if (argv.length !== 3 || argv[1] !== "--artifacts") {
      throw new Error("usage: native-packages.mjs stage --artifacts <directory>");
    }
    await stageNativePackages({ artifacts: argument(argv, "--artifacts") });
    return;
  }
  throw new Error(
    "usage: native-packages.mjs <sync <version> | stage --artifacts <directory>>",
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}

export { artifactName, main, onnxArtifactName };
