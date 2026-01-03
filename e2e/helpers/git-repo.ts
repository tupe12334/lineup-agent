import { execFile } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export interface GitRepoOptions {
  withClaudeSettings?: boolean;
  claudeSettingsContent?: object;
  withPackageJson?: boolean;
  withHusky?: boolean;
  withCargoToml?: boolean;
}

export async function createGitRepo(
  baseDir: string,
  name: string,
  options: GitRepoOptions = {}
): Promise<string> {
  const repoPath = join(baseDir, name);

  await mkdir(repoPath, { recursive: true });
  await execFileAsync("git", ["init"], { cwd: repoPath });
  await execFileAsync("git", ["config", "user.email", "test@test.com"], {
    cwd: repoPath,
  });
  await execFileAsync("git", ["config", "user.name", "Test User"], {
    cwd: repoPath,
  });

  if (options.withClaudeSettings) {
    const claudeDir = join(repoPath, ".claude");
    await mkdir(claudeDir, { recursive: true });
    const content = options.claudeSettingsContent || {
      hooks: {
        PreToolUse: [
          { matcher: "Bash", hooks: [{ type: "command", command: "echo test" }] },
        ],
      },
    };
    await writeFile(
      join(claudeDir, "settings.json"),
      JSON.stringify(content, null, 2)
    );
  }

  if (options.withPackageJson) {
    const packageJson = {
      name: name,
      version: "1.0.0",
      scripts: options.withHusky ? { prepare: "husky" } : {},
    };
    await writeFile(
      join(repoPath, "package.json"),
      JSON.stringify(packageJson, null, 2)
    );
  }

  if (options.withCargoToml) {
    const cargoToml = `[package]
name = "${name}"
version = "0.1.0"
edition = "2021"
`;
    await writeFile(join(repoPath, "Cargo.toml"), cargoToml);
  }

  if (options.withHusky) {
    await mkdir(join(repoPath, ".husky"), { recursive: true });
    await writeFile(
      join(repoPath, ".husky", "pre-commit"),
      '#!/bin/sh\necho "pre-commit"'
    );
  }

  return repoPath;
}
