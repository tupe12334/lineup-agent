import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("lint --json", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("lint-json");
  });

  it("should output valid JSON", async () => {
    const repoPath = await createGitRepo(testDir, "repo", {
      withClaudeSettings: true,
    });
    const result = await lint(repoPath, { json: true });

    expect(result.exitCode).toBe(0);
    expect(() => JSON.parse(result.stdout)).not.toThrow();
  });

  it("should have correct report structure", async () => {
    const repoPath = await createGitRepo(testDir, "repo");
    const result = await lint(repoPath, { json: true });

    const report = JSON.parse(result.stdout);

    expect(report).toHaveProperty("results");
    expect(report).toHaveProperty("errorCount");
    expect(report).toHaveProperty("warningCount");
    expect(report).toHaveProperty("infoCount");
    expect(report).toHaveProperty("fixedCount");
    expect(Array.isArray(report.results)).toBe(true);
  });

  it("should have correct result item structure", async () => {
    const repoPath = await createGitRepo(testDir, "repo");
    const result = await lint(repoPath, { json: true });

    const report = JSON.parse(result.stdout);

    expect(report.results.length).toBeGreaterThan(0);

    const firstResult = report.results[0];
    expect(firstResult).toHaveProperty("ruleId");
    expect(firstResult).toHaveProperty("severity");
    expect(firstResult).toHaveProperty("message");
    expect(firstResult).toHaveProperty("path");
    expect(["error", "warning", "info"]).toContain(firstResult.severity);
  });

  it("should count errors correctly", async () => {
    const repoPath = await createGitRepo(testDir, "repo");
    const result = await lint(repoPath, { json: true });

    const report = JSON.parse(result.stdout);
    const errorCount = report.results.filter(
      (r: { severity: string }) => r.severity === "error"
    ).length;

    expect(report.errorCount).toBe(errorCount);
  });

  it("should include suggestion when available", async () => {
    const repoPath = await createGitRepo(testDir, "repo");
    const result = await lint(repoPath, { json: true });

    const report = JSON.parse(result.stdout);
    const resultWithSuggestion = report.results.find(
      (r: { suggestion?: string }) => r.suggestion
    );

    expect(resultWithSuggestion).toBeDefined();
    expect(typeof resultWithSuggestion.suggestion).toBe("string");
  });
});
