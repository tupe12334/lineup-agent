export {
  native,
  type EngineInstance,
  type LintReport,
  type LintResult,
  type RuleInfo,
} from "./native.js";

import { native, type LintReport } from "./native.js";

/**
 * Create a lineup-agent engine
 */
export function createLineupAgent() {
  return native.createEngine("{}");
}

/**
 * Run all linting rules on the specified path
 */
export function lint(path: string): LintReport {
  const engine = createLineupAgent();
  return engine.lint(path);
}

/**
 * Run all linting rules with auto-fix on the specified path
 */
export function fix(path: string): LintReport {
  const engine = createLineupAgent();
  return engine.fix(path);
}
