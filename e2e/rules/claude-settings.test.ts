import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("claude-settings-hooks rule", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("claude-settings");
  });

  describe("detection", () => {
    it("should detect missing .claude directory", async () => {
      const repoPath = await createGitRepo(testDir, "no-claude");
      const result = await lint(repoPath, { json: true });

      const report = JSON.parse(result.stdout);
      const claudeError = report.results.find(
        (r: { ruleId: string }) => r.ruleId === "claude-settings-hooks"
      );

      expect(claudeError).toBeDefined();
      expect(claudeError.severity).toBe("error");
      expect(claudeError.message).toContain(".claude");
    });

    it("should detect missing settings.json", async () => {
      const repoPath = await createGitRepo(testDir, "no-settings");
      await mkdir(join(repoPath, ".claude"), { recursive: true });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const error = report.results.find(
        (r: { ruleId: string; message: string }) =>
          r.ruleId === "claude-settings-hooks" &&
          r.message.includes("settings.json")
      );
      expect(error).toBeDefined();
    });

    it("should detect invalid JSON in settings.json", async () => {
      const repoPath = await createGitRepo(testDir, "invalid-json");
      const claudeDir = join(repoPath, ".claude");
      await mkdir(claudeDir, { recursive: true });
      await writeFile(join(claudeDir, "settings.json"), "{ invalid json }");

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const error = report.results.find(
        (r: { ruleId: string; message: string }) =>
          r.ruleId === "claude-settings-hooks" && r.message.includes("Invalid")
      );
      expect(error).toBeDefined();
    });

    it("should detect missing hooks object", async () => {
      const repoPath = await createGitRepo(testDir, "no-hooks", {
        withClaudeSettings: true,
        claudeSettingsContent: { apiKey: "test" },
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const error = report.results.find(
        (r: { ruleId: string; message: string }) =>
          r.ruleId === "claude-settings-hooks" && r.message.includes("hooks")
      );
      expect(error).toBeDefined();
    });

    it("should detect missing Bash hook", async () => {
      const settings = {
        hooks: {
          PreToolUse: [{ matcher: "Write", hooks: [] }],
        },
      };
      const repoPath = await createGitRepo(testDir, "no-bash-hook", {
        withClaudeSettings: true,
        claudeSettingsContent: settings,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const warning = report.results.find(
        (r: { ruleId: string; message: string }) =>
          r.ruleId === "claude-settings-hooks" && r.message.includes("Bash")
      );
      expect(warning).toBeDefined();
    });

    it("should pass when properly configured", async () => {
      const repoPath = await createGitRepo(testDir, "valid", {
        withClaudeSettings: true,
      });
      const result = await lint(repoPath, { json: true });

      const report = JSON.parse(result.stdout);
      const claudeErrors = report.results.filter(
        (r: { ruleId: string }) => r.ruleId === "claude-settings-hooks"
      );

      expect(claudeErrors).toHaveLength(0);
    });
  });

  describe("nested repositories", () => {
    it("should find git repos in subdirectories", async () => {
      const parentDir = join(testDir, "parent");
      await mkdir(parentDir, { recursive: true });

      await createGitRepo(parentDir, "repo1");
      await createGitRepo(parentDir, "repo2");

      const result = await lint(parentDir, { json: true });
      const report = JSON.parse(result.stdout);

      expect(report.errorCount).toBeGreaterThanOrEqual(2);
    });
  });
});
