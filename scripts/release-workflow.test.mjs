import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { describe, it } from "node:test";

async function workflows() {
  const [release, ci] = await Promise.all([
    readFile(".github/workflows/release.yml", "utf8"),
    readFile(".github/workflows/ci.yml", "utf8"),
  ]);
  return { release, ci };
}

describe("release workflow", () => {
  it("validates native packages and the publishing bootstrap", async () => {
    const { release, ci } = await workflows();

    for (const workflow of [release, ci]) {
      assert.match(workflow, /pnpm run check:native-packages/);
      assert.match(workflow, /pnpm run check:bootstrap-npm-publishing/);
      assert.doesNotMatch(workflow, /check:native-installer/);
      for (const install of workflow.match(/^\s*npm install --ignore-scripts.*$/gm) ?? []) {
        assert.match(install, /--omit=optional/);
      }
    }
  });

  it("uses immutable action references and bounded job timeouts", async () => {
    const { release, ci } = await workflows();

    for (const workflow of [release, ci]) {
      const useLines = workflow.match(/^\s*(?:- )?uses: [^\n]+$/gm) ?? [];
      assert.ok(useLines.length > 0);
      for (const line of useLines) {
        assert.match(
          line,
          /^\s*(?:- )?uses: [^\s#]+@[a-f0-9]{40} # \S.*$/,
        );
      }
      for (const timeout of workflow.matchAll(/timeout-minutes: (\d+)/g)) {
        assert.ok(Number(timeout[1]) <= 30);
      }
    }
    assert.match(ci, /group: ci-\$\{\{ github\.event\.pull_request\.number \|\| github\.ref \}\}/);
    assert.match(ci, /cancel-in-progress: \$\{\{ github\.event_name == 'pull_request' \}\}/);
  });

  it("stages and publishes native packages through npm trusted publishing", async () => {
    const { release } = await workflows();
    const releaseCreate = release.indexOf(
      'gh release create "$tag" dist/* --title "$tag" --generate-notes',
    );
    const nativePublish = release.indexOf("Publish native platform packages");
    const primaryPublish = release.indexOf("Publish primary packages");

    assert.match(release, /node scripts\/native-packages\.mjs sync "\$version"/);
    assert.match(release, /node scripts\/native-packages\.mjs stage --artifacts dist/);
    assert.match(release, /npm install --global npm@11\.19\.0/);
    assert.match(release, /npm install --prefix "\$consumer_dir" --ignore-scripts/);
    assert.match(release, /\(cd "\$consumer_dir" && npm init -y/);
    assert.doesNotMatch(release, /npm init --prefix/);
    assert.match(release, /packages\/\{ai,html,markdown\}/);
    assert.match(release, /require\(packageRoot\('vurst-html'\)\)/);
    assert.match(release, /require\(packageRoot\('vurst-markdown'\)\)/);
    assert.match(release, /require\(packageRoot\('vurst-ai'\)\)/);
    assert.match(release, /npm publish "\.\/\$package_dir" --access public --dry-run/);
    assert.match(release, /grep -qE 'E404\|404 Not Found'/);
    assert.match(release, /npm publish "\.\/\$package_dir" --access public --provenance/);
    assert.match(release, /id-token: write/);
    assert.doesNotMatch(release, /NODE_AUTH_TOKEN|NPM_TOKEN|pnpm publish/);
    assert.ok(releaseCreate >= 0);
    assert.ok(nativePublish > releaseCreate);
    assert.ok(primaryPublish > nativePublish);
  });

  it("retains checksummed GitHub Release assets", async () => {
    const { release } = await workflows();

    assert.match(release, /name: release-assets-\$\{\{ matrix\.target \}\}/);
    assert.match(release, /shasum -a 256 "\$artifact"/);
    assert.match(release, /gh release upload "\$tag" dist\/\* --clobber/);
    assert.doesNotMatch(release, /VURST_RELEASE_BASE_URL|VURST_SKIP_BINARY_DOWNLOAD/);
  });
});
