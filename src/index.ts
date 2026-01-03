export {
  native,
  type EngineInstance,
  type LintReport,
  type LintResult,
  type RuleInfo,
} from "./native.js";

import { native, type LintReport } from "./native.js";

export interface LineupConfig {
  rules?: Record<string, RuleConfig>;
}

export interface RuleConfig {
  enabled?: boolean;
  severity?: "error" | "warning" | "info";
  options?: Record<string, unknown>;
}

/**
 * Create a lineup-agent engine with the given configuration
 */
export function createLineupAgent(config: LineupConfig = {}) {
  return native.createEngine(JSON.stringify(config));
}

/**
 * Run linting on the specified path
 */
export function lint(path: string, config?: LineupConfig): LintReport {
  const engine = createLineupAgent(config);
  return engine.lint(path);
}

/**
 * Run linting with auto-fix on the specified path
 */
export function fix(path: string, config?: LineupConfig): LintReport {
  const engine = createLineupAgent(config);
  return engine.fix(path);
}
