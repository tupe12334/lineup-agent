import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("lint command", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("lint");
  });

  describe("basic functionality", () => {
    it("should lint current directory by default", async () => {
      await createGitRepo(testDir, "repo", { withClaudeSettings: true });
      const result = await lint(".", { cwd: `${testDir}/repo` });
      expect(result.exitCode).toBe(0);
    });

    it("should lint specified path", async () => {
      const repoPath = await createGitRepo(testDir, "repo", {
        withClaudeSettings: true,
      });
      const result = await lint(repoPath);
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain("No issues found");
    });

    it("should exit with code 1 when errors found", async () => {
      const repoPath = await createGitRepo(testDir, "repo-no-settings");
      const result = await lint(repoPath);
      expect(result.exitCode).toBe(1);
      expect(result.stdout).toContain("error");
    });

    it("should handle non-existent path gracefully", async () => {
      const result = await lint("/nonexistent/path/that/does/not/exist");
      expect(result.exitCode).toBe(1);
    });
  });

  describe("output format", () => {
    it("should display severity icons correctly", async () => {
      const repoPath = await createGitRepo(testDir, "repo");
      const result = await lint(repoPath);
      expect(result.stdout).toMatch(/x|!|i/);
    });

    it("should show path in output", async () => {
      const repoPath = await createGitRepo(testDir, "repo");
      const result = await lint(repoPath);
      expect(result.stdout).toContain(repoPath);
    });
  });
});
