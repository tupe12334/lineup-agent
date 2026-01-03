import { readFile, readdir, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";
import { lint } from "../helpers/cli";
import { createGitRepo } from "../helpers/git-repo";
import { createTestDir } from "../setup";

interface FileSnapshot {
  [filePath: string]: string;
}

const SKIP_DIRS = new Set([".git", "node_modules", ".pnpm"]);

async function collectFiles(
  dir: string,
  baseDir: string = dir
): Promise<FileSnapshot> {
  const snapshot: FileSnapshot = {};
  const entries = await readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    const relativePath = relative(baseDir, fullPath);

    if (SKIP_DIRS.has(entry.name)) continue;

    if (entry.isDirectory()) {
      const subFiles = await collectFiles(fullPath, baseDir);
      Object.assign(snapshot, subFiles);
    } else {
      const content = await readFile(fullPath, "utf-8");
      snapshot[relativePath] = content;
    }
  }

  return snapshot;
}

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
    }, 30000);
  });

  describe("fix snapshots", () => {
    it("should create cspell.json with default config", async () => {
      const repoPath = await createGitRepo(testDir, "snapshot-create", {
        withPackageJson: true,
        withHusky: true,
        withClaudeSettings: true,
      });

      // Set up package.json with husky
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test-project",
          scripts: { prepare: "husky" },
        })
      );

      // Run lint with fix
      await lint(repoPath, { fix: true });

      // Collect and snapshot files
      const files = await collectFiles(repoPath);
      const sortedFiles: FileSnapshot = {};
      for (const key of Object.keys(files).sort()) {
        sortedFiles[key] = files[key];
      }

      expect(sortedFiles).toMatchSnapshot("cspell-fix-all");
    }, 30000);

    it("should add cspell dependency to package.json", async () => {
      const repoPath = await createGitRepo(testDir, "snapshot-dep", {
        withPackageJson: true,
        withHusky: true,
        withClaudeSettings: true,
      });

      // Create cspell.json but no dependency
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify({ version: "0.2", words: ["existing"] }, null, 2)
      );

      // Set up package.json without cspell
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test-project",
          devDependencies: { typescript: "^5.0.0" },
          scripts: { prepare: "husky" },
        })
      );

      // Run lint with fix
      await lint(repoPath, { fix: true });

      // Snapshot just the package.json
      const packageJson = await readFile(join(repoPath, "package.json"), "utf-8");
      expect(JSON.parse(packageJson)).toMatchSnapshot("package-json-with-cspell");
    }, 30000);

    it("should add cspell to pre-commit hook", async () => {
      const repoPath = await createGitRepo(testDir, "snapshot-hook", {
        withPackageJson: true,
        withHusky: true,
        withClaudeSettings: true,
      });

      // Create cspell.json
      await writeFile(
        join(repoPath, "cspell.json"),
        JSON.stringify({ version: "0.2" })
      );

      // Set up package.json with cspell
      await writeFile(
        join(repoPath, "package.json"),
        JSON.stringify({
          name: "test-project",
          devDependencies: { cspell: "^8.0.0" },
          scripts: { prepare: "husky" },
        })
      );

      // Run lint with fix
      await lint(repoPath, { fix: true });

      // Snapshot the pre-commit hook
      const preCommit = await readFile(join(repoPath, ".husky", "pre-commit"), "utf-8");
      expect(preCommit).toMatchSnapshot("pre-commit-with-cspell");
    }, 30000);
  });
});
