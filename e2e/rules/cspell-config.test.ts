import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

describe("cspell-config rule", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTestDir("cspell-config");
  });

  describe("detection", () => {
    it("should detect missing cspell.json", async () => {
      const repoPath = await createGitRepo(testDir, "no-cspell", {
        withPackageJson: true,
      });

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const cspellError = report.results.find(
        (r: { ruleId: string; checkId: string }) =>
          r.ruleId === "cspell-config" && r.checkId === "cspell-json-exists"
      );

      expect(cspellError).toBeDefined();
      expect(cspellError.message).toContain("cspell");
    });

    it("should detect missing cspell dependency", async () => {
      const repoPath = await createGitRepo(testDir, "no-cspell-dep", {
        withPackageJson: true,
      });

      // Create cspell.json but no dependency
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify({ version: "0.2" })
      );

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const depError = report.results.find(
        (r: { ruleId: string; checkId: string }) =>
          r.ruleId === "cspell-config" && r.checkId === "cspell-dependency"
      );

      expect(depError).toBeDefined();
      expect(depError.message).toContain("devDependencies");
    });

    it("should detect missing cspell in pre-commit hook", async () => {
      const repoPath = await createGitRepo(testDir, "no-cspell-hook", {
        withPackageJson: true,
        withHusky: true,
      });

      // Add cspell.json and cspell dependency
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify({ version: "0.2" })
      );
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test",
          devDependencies: { cspell: "^8.0.0" },
        })
      );

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const hookWarning = report.results.find(
        (r: { ruleId: string; checkId: string }) =>
          r.ruleId === "cspell-config" && r.checkId === "cspell-pre-commit-hook"
      );

      expect(hookWarning).toBeDefined();
      expect(hookWarning.message).toContain("cspell");
    });

    it("should pass when cspell is fully configured", async () => {
      const repoPath = await createGitRepo(testDir, "valid-cspell", {
        withPackageJson: true,
        withHusky: true,
      });

      // Add cspell.json
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify({ version: "0.2" })
      );

      // Add cspell dependency
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test",
          devDependencies: { cspell: "^8.0.0" },
          scripts: { prepare: "husky" },
        })
      );

      // Add cspell to pre-commit hook
      await writeFile(
        join(repoPath, ".husky", "pre-commit"),
        '#!/bin/sh\npnpm exec cspell --no-progress "**/*"'
      );

      const result = await lint(repoPath, { json: true });
      const report = JSON.parse(result.stdout);

      const cspellErrors = report.results.filter(
        (r: { ruleId: string }) => r.ruleId === "cspell-config"
      );

      expect(cspellErrors).toHaveLength(0);
    });
  });

  describe("fix behavior", () => {
    it("should not override existing cspell.json with custom config", async () => {
      const repoPath = await createGitRepo(testDir, "preserve-cspell", {
        withPackageJson: true,
        withHusky: true,
        withClaudeSettings: true,
      });

      // Add cspell dependency to package.json so that check passes
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test",
          devDependencies: { cspell: "^8.0.0" },
          scripts: { prepare: "husky" },
        })
      );

      // Add cspell to pre-commit so that check passes
      await writeFile(
        join(repoPath, ".husky", "pre-commit"),
        '#!/bin/sh\npnpm exec cspell "**/*"'
      );

      // Create custom cspell.json with user-specific configuration
      const customConfig = {
        version: "0.2",
        language: "en",
        words: ["mycompany", "customword", "specialterm"],
        ignorePaths: ["my-custom-path", "another-path"],
        dictionaries: ["custom-dict"],
      };
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify(customConfig, null, 2)
      );

      // Run lint with fix
      await lint(repoPath, { fix: true });

      // Read the cspell.json after fix
      const content = await readFile(join(repoPath, "cspell.json"), "utf-8");
      const configAfterFix = JSON.parse(content);

      // Verify custom config is preserved
      expect(configAfterFix.words).toContain("mycompany");
      expect(configAfterFix.words).toContain("customword");
      expect(configAfterFix.words).toContain("specialterm");
      expect(configAfterFix.ignorePaths).toContain("my-custom-path");
      expect(configAfterFix.ignorePaths).toContain("another-path");
      expect(configAfterFix.dictionaries).toContain("custom-dict");

      // Verify it wasn't replaced with default config
      expect(configAfterFix.ignorePaths).not.toContain("node_modules");
    });
  });
});
