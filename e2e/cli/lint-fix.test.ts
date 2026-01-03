import { access, readFile } from "node:fs/promises";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("lint --fix", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("lint-fix");
  });

  it("should create .claude/settings.json when missing", async () => {
    // Create repo without package.json so husky-init rule doesn't trigger
    const repoPath = await createGitRepo(testDir, "repo");

    const result = await lint(repoPath, { fix: true });

    // Note: May still exit 1 if husky-init has unfixable issues, but fix should work
    expect(result.stdout).toContain("fixed");

    const settingsPath = join(repoPath, ".claude", "settings.json");
    await expect(access(settingsPath)).resolves.toBeUndefined();

    const content = JSON.parse(await readFile(settingsPath, "utf-8"));
    expect(content.hooks).toBeDefined();
    expect(content.hooks.PreToolUse).toBeDefined();
  });

  it("should preserve existing settings when merging hooks", async () => {
    const existingSettings = {
      apiKey: "test-key",
      customSetting: { nested: "value" },
    };
    const repoPath = await createGitRepo(testDir, "repo", {
      withClaudeSettings: true,
      claudeSettingsContent: existingSettings,
    });

    await lint(repoPath, { fix: true });

    const settingsPath = join(repoPath, ".claude", "settings.json");
    const content = JSON.parse(await readFile(settingsPath, "utf-8"));

    expect(content.apiKey).toBe("test-key");
    expect(content.customSetting.nested).toBe("value");
    expect(content.hooks.PreToolUse).toBeDefined();
  });

  it("should not duplicate existing Bash hook", async () => {
    const existingSettings = {
      hooks: {
        PreToolUse: [
          { matcher: "Bash", hooks: [{ type: "command", command: "existing" }] },
        ],
      },
    };
    const repoPath = await createGitRepo(testDir, "repo", {
      withClaudeSettings: true,
      claudeSettingsContent: existingSettings,
    });

    const resultBefore = await lint(repoPath, { fix: true });
    expect(resultBefore.stdout).not.toContain("fixed");

    const settingsPath = join(repoPath, ".claude", "settings.json");
    const content = JSON.parse(await readFile(settingsPath, "utf-8"));

    const bashHooks = content.hooks.PreToolUse.filter(
      (h: { matcher: string }) => h.matcher === "Bash"
    );
    expect(bashHooks).toHaveLength(1);
    expect(bashHooks[0].hooks[0].command).toBe("existing");
  });

  it("should report fixed count in output", async () => {
    const repoPath = await createGitRepo(testDir, "repo");
    const result = await lint(repoPath, { fix: true });
    expect(result.stdout).toMatch(/\d+ fixed/);
  });
});
