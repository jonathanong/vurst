import { execFile } from "node:child_process";
import {
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { pathToFileURL } from "node:url";
import { NATIVE_PACKAGES } from "./native-packages.mjs";

const execFileAsync = promisify(execFile);

export const MIN_NPM_VERSION = Object.freeze([11, 15, 0]);
export const TRUST_REPOSITORY = "jonathanong/vurst";
export const TRUST_WORKFLOW = "release.yml";
export const PLATFORM_PACKAGES = Object.freeze(
  NATIVE_PACKAGES.map(({ name }) => name),
);

function commandError(command, args, error, otp) {
  const safeArguments = args.map((argument) => redact(argument, otp)).join(" ");
  const detail = redact(
    String(error?.stderr || error?.stdout || error?.message || ""),
    otp,
  ).trim();
  const safe = new Error(
    `${command} ${safeArguments || "command"} failed${
      error?.code ? ` (${error.code})` : ""
    }${detail ? `\n${detail}` : ""}`,
  );
  safe.code = error?.code;
  safe.stdout = redact(String(error?.stdout ?? ""), otp);
  safe.stderr = redact(String(error?.stderr ?? ""), otp);
  return safe;
}

function redact(value, otp) {
  return otp && value ? value.split(otp).join("[redacted]") : value;
}

function otpFromArgs(args) {
  const otpArgument = args.find((argument) => argument.startsWith("--otp="));
  return otpArgument?.slice("--otp=".length);
}

/**
 * The default runner deliberately does not use a shell. This keeps the OTP
 * out of shell interpolation and lets tests inject a deterministic runner.
 */
export async function runCommand(command, args, options = {}) {
  try {
    return await execFileAsync(command, args, {
      cwd: options.cwd,
      maxBuffer: 10 * 1024 * 1024,
    });
  } catch (error) {
    throw commandError(command, args, error, otpFromArgs(args));
  }
}

export function parseOtp(argv) {
  if (argv.length !== 1 || argv[0].length === 0 || argv[0].startsWith("-")) {
    throw new Error("Usage: node scripts/bootstrap-npm-publishing.mjs <otp>");
  }
  return argv[0];
}

export function parseArguments(argv) {
  if (argv.length < 1 || argv[0].length === 0 || argv[0].startsWith("-")) {
    throw new Error(
      "Usage: node scripts/bootstrap-npm-publishing.mjs <otp> [package ...]",
    );
  }
  const [otp, ...requestedPackages] = argv;
  const packageNames = requestedPackages.length
    ? [...new Set(requestedPackages)]
    : [...PLATFORM_PACKAGES];
  for (const packageName of packageNames) {
    if (!PLATFORM_PACKAGES.includes(packageName)) {
      throw new Error(`Unknown Vurst platform package: ${packageName}`);
    }
  }
  return { otp, packageNames };
}

function parseVersion(version) {
  const match = String(version).trim().match(/^(\d+)\.(\d+)(?:\.(\d+))?$/);
  if (!match) {
    throw new Error(`Could not parse npm version: ${String(version).trim()}`);
  }
  return [Number(match[1]), Number(match[2]), Number(match[3] ?? 0)];
}

export function isSupportedNpmVersion(version) {
  const actual = parseVersion(version);
  for (let index = 0; index < MIN_NPM_VERSION.length; index += 1) {
    if (actual[index] !== MIN_NPM_VERSION[index]) {
      return actual[index] > MIN_NPM_VERSION[index];
    }
  }
  return true;
}

function outputOf(result) {
  if (typeof result === "string") return result;
  return String(result?.stdout ?? "");
}

function failureOutput(error) {
  return `${error?.code ?? ""} ${error?.stdout ?? ""} ${error?.stderr ?? ""} ${
    error?.message ?? ""
  }`;
}

function isNotFoundError(error) {
  return /(?:E404|404\s+(?:Not Found|not found)|is not in this registry|\bnot found\b)/i.test(
    failureOutput(error),
  );
}

function isAlreadyPublishedError(error) {
  return /You cannot publish over the previously published versions:\s*0\.0\.0/i.test(
    failureOutput(error),
  );
}

async function npm(run, args, otp) {
  try {
    const result = await run("npm", args);
    if (result && typeof result.exitCode === "number" && result.exitCode !== 0) {
      throw commandError("npm", args, result, otp);
    }
    return result ?? { stdout: "", stderr: "" };
  } catch (error) {
    throw commandError("npm", args, error, otp);
  }
}

async function packageExists(run, packageName, otp) {
  try {
    await npm(run, ["view", packageName, "version", "--json"], otp);
    return true;
  } catch (error) {
    if (isNotFoundError(error)) return false;
    throw error;
  }
}

async function createStub(packageName, root) {
  const directory = await mkdtemp(join(root, "vurst-npm-bootstrap-"));
  const license = await readFile(new URL("../LICENSE", import.meta.url), "utf8");
  await writeFile(
    join(directory, "package.json"),
    `${JSON.stringify(
      {
        name: packageName,
        version: "0.0.0",
        description: "Temporary bootstrap package for Vurst trusted publishing",
        private: false,
        license: "MIT",
        engines: { node: ">= 18" },
        repository: {
          type: "git",
          url: "git+https://github.com/jonathanong/vurst.git",
          directory: `packages/${packageName.replace("@jongleberry/", "")}`,
        },
        main: "index.js",
        files: ["index.js", "README.md", "LICENSE"],
        publishConfig: { access: "public" },
      },
      null,
      2,
    )}\n`,
  );
  await writeFile(join(directory, "index.js"), "module.exports = {};\n");
  await writeFile(
    join(directory, "README.md"),
    `# ${packageName}\n\nBootstrap placeholder.\n`,
  );
  await writeFile(join(directory, "LICENSE"), license);
  return directory;
}

function trustRecords(output) {
  const text = String(output ?? "").trim();
  if (!text) return [];

  try {
    const parsed = JSON.parse(text);
    if (parsed == null) return [];
    return Array.isArray(parsed) ? parsed : [parsed];
  } catch {
    // `--json` is supported by npm 11.15+, but treating non-empty text as a
    // relationship is safer than creating a second relationship blindly.
    return [{ __unparsed: text }];
  }
}

function isExactTrustRecord(record) {
  if (!record || record.__unparsed) return false;
  const claims = record.claims ?? record;
  const workflow = claims.workflow_ref ?? claims.workflowRef ?? {};
  const permissions = record.permissions ?? claims.permissions ?? [];
  const permissionList = Array.isArray(permissions)
    ? permissions
    : [permissions];
  const repository = claims.repository ?? record.repository;
  const file = workflow.file ?? claims.file ?? record.file;
  return (
    String(record.type ?? claims.type ?? "").toLowerCase() === "github" &&
    repository === TRUST_REPOSITORY &&
    file === TRUST_WORKFLOW &&
    permissionList.some((permission) =>
      ["createPackage", "publish"].includes(String(permission)),
    )
  );
}

export function inspectTrust(output) {
  const records = trustRecords(output);
  if (records.length === 0) return "empty";
  if (records.every(isExactTrustRecord)) return "exact";
  return "conflict";
}

async function trustList(run, packageName, otp) {
  const result = await npm(
    run,
    ["trust", "list", packageName, "--json", `--otp=${otp}`],
    otp,
  );
  return outputOf(result);
}

async function ensureTrust(run, packageName, otp, preflightStatus) {
  const before =
    preflightStatus ?? inspectTrust(await trustList(run, packageName, otp));
  if (before === "conflict") {
    throw new Error(
      `Conflicting npm trust relationship exists for ${packageName}; revoke it manually before rerunning`,
    );
  }
  if (before === "empty") {
    await npm(
      run,
      [
        "trust",
        "github",
        packageName,
        "--repo",
        TRUST_REPOSITORY,
        "--file",
        TRUST_WORKFLOW,
        "--allow-publish",
        "--yes",
        `--otp=${otp}`,
      ],
      otp,
    );
  }
}

export async function bootstrap({
  otp,
  packageNames = PLATFORM_PACKAGES,
  run = runCommand,
  temporaryRoot = tmpdir(),
} = {}) {
  if (typeof otp !== "string" || otp.length === 0) {
    throw new Error("An OTP is required as the only positional argument");
  }
  if (!Array.isArray(packageNames) || packageNames.length === 0) {
    throw new Error("At least one Vurst platform package is required");
  }
  for (const packageName of packageNames) {
    if (!PLATFORM_PACKAGES.includes(packageName)) {
      throw new Error(`Unknown Vurst platform package: ${packageName}`);
    }
  }

  const versionResult = await npm(run, ["--version"], otp);
  const npmVersion = outputOf(versionResult).trim();
  if (!isSupportedNpmVersion(npmVersion)) {
    throw new Error(
      `npm ${npmVersion} is too old; npm 11.15.0 or newer is required`,
    );
  }

  await npm(run, ["whoami"], otp);

  const temporaryDirectories = [];
  try {
    const packageStates = [];
    for (const packageName of packageNames) {
      const exists = await packageExists(run, packageName, otp);
      let trustStatus;
      if (exists) {
        trustStatus = inspectTrust(await trustList(run, packageName, otp));
        if (trustStatus === "conflict") {
          throw new Error(
            `Conflicting npm trust relationship exists for ${packageName}; revoke it manually before rerunning`,
          );
        }
      }
      packageStates.push({ packageName, exists, trustStatus });
    }

    for (const { packageName, exists, trustStatus } of packageStates) {
      let effectiveTrustStatus = trustStatus;
      if (!exists) {
        const directory = await createStub(packageName, temporaryRoot);
        temporaryDirectories.push(directory);
        await npm(
          run,
          ["publish", directory, "--access", "public", "--dry-run"],
          otp,
        );
        try {
          await npm(
            run,
            ["publish", directory, "--access", "public", `--otp=${otp}`],
            otp,
          );
          effectiveTrustStatus = "empty";
        } catch (error) {
          if (!isAlreadyPublishedError(error)) throw error;
          // A prior run can publish 0.0.0 before registry reads expose it.
          // Inspect trust instead of trying to overwrite the immutable version.
          effectiveTrustStatus = undefined;
        }
      }
      await ensureTrust(run, packageName, otp, effectiveTrustStatus);
    }

    // A final read-only trust pass verifies every package without depending
    // on the eventually consistent public package view endpoint.
    for (const packageName of packageNames) {
      const status = inspectTrust(await trustList(run, packageName, otp));
      if (status !== "exact") {
        throw new Error(`Could not verify exact npm trust for ${packageName}`);
      }
    }
  } finally {
    await Promise.all(
      temporaryDirectories.map((directory) =>
        rm(directory, { recursive: true, force: true }),
      ),
    );
  }
}

async function main() {
  const { otp, packageNames } = parseArguments(process.argv.slice(2));
  try {
    await bootstrap({ otp, packageNames });
    console.log(
      `Bootstrapped ${packageNames.length} npm packages and verified OIDC trust.`,
    );
  } catch (error) {
    const message = redact(error?.message ?? String(error), otp);
    process.stderr.write(`${message}\n`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
