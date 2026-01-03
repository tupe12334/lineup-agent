import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("husky-init rule", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("husky-init");
  });

  describe("JavaScript projects", () => {
    it("should detect missing .husky directory", async () => {
      const repoPath = await createGitRepo(testDir, "no-husky", {
        withClaudeSettings: true,
        withPackageJson: true,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const huskyIssue = report.results.find(
        (r: { ruleId: string }) => r.ruleId === "husky-init"
      );
      expect(huskyIssue).toBeDefined();
    });

    it("should detect missing prepare script", async () => {
      const repoPath = await createGitRepo(testDir, "no-prepare", {
        withClaudeSettings: true,
        withPackageJson: true,
      });
      await mkdir(join(repoPath, ".husky"), { recursive: true });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const warning = report.results.find(
        (r: { ruleId: string; message: string }) =>
          r.ruleId === "husky-init" && r.message.includes("prepare")
      );
      expect(warning).toBeDefined();
    });

    it("should pass when husky is properly configured", async () => {
      const repoPath = await createGitRepo(testDir, "valid-husky", {
        withClaudeSettings: true,
        withPackageJson: true,
        withHusky: true,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const huskyErrors = report.results.filter(
        (r: { ruleId: string; severity: string }) =>
          r.ruleId === "husky-init" && r.severity === "error"
      );
      expect(huskyErrors).toHaveLength(0);
    });
  });

  describe("Rust projects", () => {
    it("should detect Rust project without husky-rs", async () => {
      const repoPath = await createGitRepo(testDir, "rust-no-husky", {
        withClaudeSettings: true,
        withCargoToml: true,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const huskyIssue = report.results.find(
        (r: { ruleId: string }) => r.ruleId === "husky-init"
      );
      expect(huskyIssue).toBeDefined();
    });
  });

  describe("projects without manifest", () => {
    it("should not report husky issues for projects without package.json or Cargo.toml", async () => {
      const repoPath = await createGitRepo(testDir, "no-manifest", {
        withClaudeSettings: true,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const huskyIssues = report.results.filter(
        (r: { ruleId: string }) => r.ruleId === "husky-init"
      );
      expect(huskyIssues).toHaveLength(0);
    });
  });
});
