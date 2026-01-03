import { createRequire } from "module";
import { arch, platform } from "os";
import { join, dirname } from "path";
import { existsSync } from "fs";
import { fileURLToPath } from "url";

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));

interface NativeBinding {
  Engine: new (configJson: string) => EngineInstance;
  createEngine: (configJson: string) => EngineInstance;
}

export interface EngineInstance {
  lint: (path: string) => LintReport;
  fix: (path: string) => LintReport;
  listRules: () => RuleInfo[];
}

export interface LintResult {
  ruleId: string;
  severity: string;
  message: string;
  path: string;
  line?: number;
  suggestion?: string;
}

export interface LintReport {
  results: LintResult[];
  errorCount: number;
  warningCount: number;
  infoCount: number;
  fixedCount: number;
}

export interface RuleInfo {
  id: string;
  name: string;
  description: string;
  defaultSeverity: string;
  canFix: boolean;
}

function getPackageName(): string {
  const platformName = platform();
  const archName = arch();

  const platformArchMap: Record<string, Record<string, string>> = {
    darwin: {
      arm64: "@lineup-agent/darwin-arm64",
      x64: "@lineup-agent/darwin-x64",
    },
    linux: {
      arm64: "@lineup-agent/linux-arm64-gnu",
      x64: "@lineup-agent/linux-x64-gnu",
    },
    win32: {
      x64: "@lineup-agent/win32-x64-msvc",
    },
  };

  const platformPackages = platformArchMap[platformName];
  if (!platformPackages) {
    throw new Error(`Unsupported platform: ${platformName}`);
  }

  const packageName = platformPackages[archName];
  if (!packageName) {
    throw new Error(`Unsupported architecture: ${archName} on ${platformName}`);
  }

  return packageName;
}

function getLocalBindingName(): string {
  const platformName = platform();
  const archName = arch();

  // NAPI-RS uses specific triple suffixes
  const suffixMap: Record<string, Record<string, string>> = {
    darwin: { arm64: "", x64: "" },
    linux: { arm64: "-gnu", x64: "-gnu" },
    win32: { x64: "-msvc" },
  };

  const suffix = suffixMap[platformName]?.[archName] ?? "";
  return `lineup-agent.${platformName}-${archName}${suffix}.node`;
}

function loadNativeBinding(): NativeBinding {
  // Try loading from npm package first (production)
  try {
    const packageName = getPackageName();
    return require(packageName);
  } catch {
    // Fallback to local .node file (development)
  }

  // Try loading from project root (development mode)
  const bindingName = getLocalBindingName();
  const localBindings = [
    join(__dirname, "..", bindingName),
    join(__dirname, "..", "..", bindingName),
  ];

  for (const bindingPath of localBindings) {
    if (existsSync(bindingPath)) {
      return require(bindingPath);
    }
  }

  throw new Error(
    `Failed to load native binding. Tried:\n` +
      `  - npm package: ${getPackageName()}\n` +
      `  - local paths: ${localBindings.join(", ")}\n` +
      `Platform: ${platform()}-${arch()}`
  );
}

export const native = loadNativeBinding();
