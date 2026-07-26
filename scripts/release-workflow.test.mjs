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

  it("publishes checksummed native assets before npm packages", async () => {
    const workflow = await readFile(".github/workflows/release.yml", "utf8");
    const releaseCreate = workflow.indexOf(
      'gh release create "$tag" dist/* --title "$tag" --generate-notes',
    );
    const firstNpmPublish = workflow.indexOf(
      "pnpm publish --access public --provenance --no-git-checks",
    );

    assert.match(workflow, /name: release-assets-\$\{\{ matrix\.target \}\}/);
    assert.match(workflow, /shasum -a 256 "\$artifact"/);
    assert.match(workflow, /gh release upload "\$tag" dist\/\* --clobber/);
    assert.match(workflow, /Smoke-test GitHub Release installers/);
    assert.ok(releaseCreate >= 0);
    assert.ok(firstNpmPublish > releaseCreate);
    assert.doesNotMatch(workflow, /pattern: bindings-(?:ai|html|markdown)-\*/);
  });
});
