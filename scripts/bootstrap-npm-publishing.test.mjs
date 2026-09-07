import assert from "node:assert/strict";
import { test } from "node:test";

import {
  PLATFORM_PACKAGES,
  bootstrap,
  inspectTrust,
  isSupportedNpmVersion,
  parseOtp,
} from "./bootstrap-npm-publishing.mjs";

const exactTrust = (packageName) =>
  JSON.stringify([
    {
      id: `trust-${packageName}`,
      type: "github",
      claims: {
        repository: "jonathanong/vurst",
        workflow_ref: { file: "release.yml" },
      },
      permissions: ["createPackage"],
    },
  ]);

function runner({ existing = false, conflict = false, conflictPackage } = {}) {
  const calls = [];
  const trusted = new Set();
  const run = async (command, args) => {
    calls.push([command, args]);
    assert.equal(command, "npm");

    if (args[0] === "--version") return { stdout: "11.19.0\n" };
    if (args[0] === "whoami") return { stdout: "maintainer\n" };
    if (args[0] === "view") {
      if (existing || calls.filter(([_, calledArgs]) => calledArgs[0] === "view").length > PLATFORM_PACKAGES.length) {
        return { stdout: '"0.4.0"\n' };
      }
      const packageName = args[1];
      const packageViewCount = calls.filter(
        ([_, calledArgs]) => calledArgs[0] === "view" && calledArgs[1] === packageName,
      ).length;
      if (packageViewCount === 1) {
        const error = new Error("not found");
        error.stderr = "npm error code E404";
        throw error;
      }
      return { stdout: '"0.0.0"\n' };
    }
    if (args[0] === "trust" && args[1] === "list") {
      if (conflict || args[2] === conflictPackage) {
        return {
          stdout: JSON.stringify([
            {
              type: "github",
              claims: {
                repository: "someone/else",
                workflow_ref: { file: "other.yml" },
              },
              permissions: ["createPackage"],
            },
          ]),
        };
      }
      return {
        stdout: existing || trusted.has(args[2]) ? exactTrust(args[2]) : "[]",
      };
    }
    if (args[0] === "trust" && args[1] === "github") {
      trusted.add(args[2]);
      return { stdout: "" };
    }
    return { stdout: "" };
  };
  return { calls, run };
}

function propagationRaceRunner() {
  const fake = runner();
  const run = async (command, args) => {
    if (args[0] === "publish" && !args.includes("--dry-run")) {
      const error = new Error("already published");
      error.stderr =
        "npm error 403 You cannot publish over the previously published versions: 0.0.0.";
      throw error;
    }
    return fake.run(command, args);
  };
  return { calls: fake.calls, run };
}

test("accepts exactly one OTP and enforces npm 11.15", () => {
  assert.equal(parseOtp(["123456"]), "123456");
  assert.throws(() => parseOtp([]), /Usage/);
  assert.throws(() => parseOtp(["one", "two"]), /Usage/);
  assert.throws(() => parseOtp(["--otp=123456"]), /Usage/);
  assert.equal(isSupportedNpmVersion("11.15.0"), true);
  assert.equal(isSupportedNpmVersion("11.19.0"), true);
  assert.equal(isSupportedNpmVersion("11.14.9"), false);
  assert.equal(isSupportedNpmVersion("10.9.0"), false);
  assert.throws(() => isSupportedNpmVersion("11.19.0-dev"), /parse npm version/);
});

test("classifies exact, empty, and conflicting trust relationships", () => {
  assert.equal(inspectTrust("[]"), "empty");
  assert.equal(inspectTrust(exactTrust(PLATFORM_PACKAGES[0])), "exact");
  assert.equal(
    inspectTrust(
      JSON.stringify([
        {
          id: "flattened-trust",
          type: "github",
          file: "release.yml",
          repository: "jonathanong/vurst",
          permissions: ["createPackage"],
        },
      ]),
    ),
    "exact",
  );
  assert.equal(
    inspectTrust(
      JSON.stringify([
        {
          type: "github",
          claims: {
            repository: "jonathanong/vurst",
            workflow_ref: { file: "other.yml" },
          },
          permissions: ["createPackage"],
        },
      ]),
    ),
    "conflict",
  );
});

test("publishes absent packages, creates exact trust, and verifies the result", async () => {
  const fake = runner();
  await bootstrap({ otp: "123456", run: fake.run });

  assert.equal(
    fake.calls.filter(([, args]) => args[0] === "publish" && args.includes("--dry-run")).length,
    PLATFORM_PACKAGES.length,
  );
  assert.equal(
    fake.calls.filter(
      ([, args]) =>
        args[0] === "publish" &&
        args.includes("--access") &&
        !args.includes("--dry-run"),
    ).length,
    PLATFORM_PACKAGES.length,
  );
  assert.equal(
    fake.calls.filter(([, args]) => args[0] === "trust" && args[1] === "github").length,
    PLATFORM_PACKAGES.length,
  );
  assert.ok(
    fake.calls
      .filter(([, args]) => args[0] === "trust" && args[1] === "list")
      .every(([, args]) => args.includes("--otp=123456")),
  );
  assert.ok(
    fake.calls.every(([, args]) => !args.some((arg) => arg === "--provenance")),
  );
});

test("skips publication and exact trust creation for existing packages", async () => {
  const fake = runner({ existing: true });
  await bootstrap({ otp: "123456", run: fake.run });

  assert.equal(fake.calls.some(([, args]) => args[0] === "publish"), false);
  assert.equal(
    fake.calls.some(([, args]) => args[0] === "trust" && args[1] === "github"),
    false,
  );
  assert.equal(
    fake.calls.filter(([, args]) => args[0] === "trust" && args[1] === "list").length,
    PLATFORM_PACKAGES.length * 2,
  );
});

test("recovers when a prior run published 0.0.0 before registry reads catch up", async () => {
  const fake = propagationRaceRunner();
  await bootstrap({ otp: "123456", run: fake.run });

  assert.equal(
    fake.calls.filter(([, args]) => args[0] === "trust" && args[1] === "github").length,
    PLATFORM_PACKAGES.length,
  );
});

test("stops before mutation when an existing package has conflicting trust", async () => {
  const fake = runner({ existing: true, conflict: true });
  await assert.rejects(
    bootstrap({ otp: "123456", run: fake.run }),
    /Conflicting npm trust relationship/,
  );
  assert.equal(fake.calls.some(([, args]) => args[0] === "publish"), false);
  assert.equal(
    fake.calls.some(([, args]) => args[0] === "trust" && args[1] === "github"),
    false,
  );
});

test("preflights every existing package before any mutation", async () => {
  const lastPackage = PLATFORM_PACKAGES.at(-1);
  const fake = runner({ existing: true, conflictPackage: lastPackage });
  await assert.rejects(
    bootstrap({ otp: "123456", run: fake.run }),
    /Conflicting npm trust relationship/,
  );
  assert.equal(fake.calls.some(([, args]) => args[0] === "publish"), false);
  assert.equal(
    fake.calls.some(([, args]) => args[0] === "trust" && args[1] === "github"),
    false,
  );
});

test("does not expose the OTP in command errors", async () => {
  const fakeRun = async (_command, args) => {
    if (args[0] === "--version") return { stdout: "11.19.0\n" };
    const error = new Error("secret 987654");
    error.stderr = "secret 987654";
    throw error;
  };
  await assert.rejects(
    bootstrap({ otp: "987654", run: fakeRun }),
    (error) =>
      !error.message.includes("987654") && error.message.includes("[redacted]"),
  );
});
