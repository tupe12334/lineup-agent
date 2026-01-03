import { execFile } from "node:child_process";
import { resolve } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

// Use process.cwd() which is the project root when running via pnpm
const CLI_PATH = resolve(process.cwd(), "dist/cli.js");

export interface CLIResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export async function runCLI(
  args: string[],
  options?: {
    cwd?: string;
    env?: NodeJS.ProcessEnv;
  }
): Promise<CLIResult> {
  try {
    const { stdout, stderr } = await execFileAsync(
      "node",
      [CLI_PATH, ...args],
      {
        cwd: options?.cwd,
        env: { ...process.env, ...options?.env },
        timeout: 20000,
      }
    );
    return { stdout, stderr, exitCode: 0 };
  } catch (error: unknown) {
    const execError = error as {
      stdout?: string;
      stderr?: string;
      code?: number;
    };
    return {
      stdout: execError.stdout || "",
      stderr: execError.stderr || "",
      exitCode: execError.code ?? 1,
    };
  }
}

export async function lint(
  path: string,
  options?: { fix?: boolean; json?: boolean; cwd?: string }
): Promise<CLIResult> {
  const args = ["lint", path];
  if (options?.fix) args.push("--fix");
  if (options?.json) args.push("--json");
  return runCLI(args, { cwd: options?.cwd });
}

export async function rules(options?: { json?: boolean }): Promise<CLIResult> {
  const args = ["rules"];
  if (options?.json) args.push("--json");
  return runCLI(args);
}
