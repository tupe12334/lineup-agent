import { execFile } from "node:child_process";
import { cp, mkdir, readdir, readFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import { promisify } from "node:util";
import { beforeAll, describe, expect, it } from "vitest";
import { createTestDir } from "../setup";

const execFileAsync = promisify(execFile);

const FIXTURES_DIR = resolve(__dirname, "../fixtures/sample-project");
const CLI_PATH = resolve(process.cwd(), "dist/cli.js");

interface FileSnapshot {
  [filePath: string]: string;
}

// Directories to skip when collecting files
const SKIP_DIRS = new Set([".git", "node_modules", ".pnpm"]);

async function getAllFiles(dir: string, baseDir: string = dir): Promise<FileSnapshot> {
  const snapshot: FileSnapshot = {};
  const entries = await readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    const relativePath = relative(baseDir, fullPath);

    // Skip certain directories
    if (SKIP_DIRS.has(entry.name)) continue;

    if (entry.isDirectory()) {
      const subFiles = await getAllFiles(fullPath, baseDir);
      Object.assign(snapshot, subFiles);
    } else {
      const content = await readFile(fullPath, "utf-8");
      snapshot[relativePath] = content;
    }
  }

  return snapshot;
}

describe("fix output snapshot", () => {
  let projectDir: string;
  let filesAfterFix: FileSnapshot;

  beforeAll(async () => {
    // Create temp directory and copy fixture
    const testDir = await createTestDir("fix-snapshot");
    projectDir = join(testDir, "sample-project");
    await mkdir(projectDir, { recursive: true });

    // Copy fixture files
    await cp(FIXTURES_DIR, projectDir, { recursive: true });

    // Initialize git repo (required for rules to run)
    await execFileAsync("git", ["init"], { cwd: projectDir });
    await execFileAsync("git", ["config", "user.email", "test@test.com"], {
      cwd: projectDir,
    });
    await execFileAsync("git", ["config", "user.name", "Test"], {
      cwd: projectDir,
    });

    // Run lint --fix
    try {
      await execFileAsync("node", [CLI_PATH, "lint", projectDir, "--fix"], {
        timeout: 30000,
      });
    } catch {
      // lint --fix may exit with error code if some issues aren't fixable
    }

    // Collect all files after fix
    filesAfterFix = await getAllFiles(projectDir);
  }, 60000); // 60s timeout for fix operations

  it("should match the file snapshot after --fix", () => {
    // Sort keys for consistent ordering
    const sortedSnapshot: FileSnapshot = {};
    for (const key of Object.keys(filesAfterFix).sort()) {
      sortedSnapshot[key] = filesAfterFix[key];
    }

    expect(sortedSnapshot).toMatchSnapshot();
  });
});
