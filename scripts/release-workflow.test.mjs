import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { describe, it } from "node:test";

describe("release workflow", () => {
  it("uses the repository-pinned pnpm for package publishing", async () => {
    const workflow = await readFile(".github/workflows/release.yml", "utf8");

    assert.equal(
      workflow.match(
        /uses: pnpm\/action-setup@0ebf47130e4866e96fce0953f49152a61190b271 # v6\.0\.9/g,
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
