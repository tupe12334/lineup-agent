import { beforeEach, describe, expect, it } from "vitest";
import { lint, runCLI } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("exit codes", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("exit-codes");
  });

  it("should exit 0 when no errors found", async () => {
    const repoPath = await createGitRepo(testDir, "clean-repo", {
      withClaudeSettings: true,
    });
    const result = await lint(repoPath);
    expect(result.exitCode).toBe(0);
  });

  it("should exit 1 when errors found", async () => {
    const repoPath = await createGitRepo(testDir, "error-repo");
    const result = await lint(repoPath);
    expect(result.exitCode).toBe(1);
  });

  it("should exit 0 after successful fix when all issues are fixable", async () => {
    // Create repo with claude settings already configured, no package.json
    // so husky-init doesn't trigger unfixable errors
    const repoPath = await createGitRepo(testDir, "fix-repo", {
      withClaudeSettings: true,
    });
    const result = await lint(repoPath, { fix: true });
    expect(result.exitCode).toBe(0);
  });

  it("should exit 0 for rules command", async () => {
    const result = await runCLI(["rules"]);
    expect(result.exitCode).toBe(0);
  });

  it("should exit 1 for invalid path", async () => {
    const result = await lint("/path/that/does/not/exist");
    expect(result.exitCode).toBe(1);
  });

  it("should exit 0 for rules --json", async () => {
    const result = await runCLI(["rules", "--json"]);
    expect(result.exitCode).toBe(0);
  });
});
