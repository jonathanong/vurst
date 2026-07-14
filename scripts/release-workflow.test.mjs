import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { describe, it } from "node:test";

describe("release workflow", () => {
  it("uses the repository-pinned pnpm for package publishing", async () => {
    const workflow = await readFile(".github/workflows/release.yml", "utf8");

    assert.equal(
      workflow.match(
        /uses: pnpm\/action-setup@b906affcce14559ad1aafd4ab0e942779e9f58b1 # v4/g,
      )?.length,
      2,
    );
    assert.equal(
      workflow.match(/pnpm publish --access public --provenance --no-git-checks/g)
        ?.length,
      5,
    );
    assert.doesNotMatch(workflow, /\bnpm publish --access public --provenance/);
  });

  it("checks packed manifests before publishing", async () => {
    const workflow = await readFile(".github/workflows/release.yml", "utf8");

    assert.match(workflow, /pnpm run check:packed-manifests/);
  });
});
