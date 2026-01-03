#!/usr/bin/env node

import { Command } from "commander";
import { native } from "./native.js";
import { resolve } from "path";
import chalk from "chalk";
import type { LintReport, LintResult } from "./native.js";

const program = new Command();

program
  .name("lineup-agent")
  .description("Rule-based linting and enforcement CLI tool")
  .version("0.1.0");

program
  .command("lint", { isDefault: true })
  .description("Run all linting rules on the specified path")
  .argument("[path]", "Path to lint", ".")
  .option("--fix", "Automatically fix issues where possible")
  .option("--json", "Output results as JSON")
  .action((targetPath: string, options: LintOptions) => {
    const engine = native.createEngine("{}");
    const absolutePath = resolve(targetPath);

    const report: LintReport = options.fix
      ? engine.fix(absolutePath)
      : engine.lint(absolutePath);

    if (options.json) {
      console.log(JSON.stringify(report, null, 2));
    } else {
      formatReport(report, options.fix);
    }

    if (report.errorCount > 0) {
      process.exit(1);
    }
  });

program
  .command("rules")
  .description("List all available rules")
  .option("--json", "Output as JSON")
  .action((options: { json?: boolean }) => {
    const engine = native.createEngine("{}");
    const rules = engine.listRules();

    if (options.json) {
      console.log(JSON.stringify(rules, null, 2));
    } else {
      console.log(chalk.bold("\nAvailable Rules:\n"));
      for (const rule of rules) {
        console.log(`  ${chalk.cyan(rule.id)}`);
        console.log(`    ${rule.description}`);
        console.log(
          `    Severity: ${rule.defaultSeverity}, Can fix: ${rule.canFix}\n`
        );
      }
    }
  });

interface LintOptions {
  fix?: boolean;
  json?: boolean;
}

function formatReport(report: LintReport, fixMode?: boolean): void {
  if (report.results.length === 0) {
    if (fixMode && report.fixedCount > 0) {
      console.log(chalk.green(`\n  All issues fixed! (${report.fixedCount} fixed)\n`));
    } else {
      console.log(chalk.green("\n  No issues found!\n"));
    }
    return;
  }

  for (const result of report.results) {
    formatResult(result);
  }

  console.log();

  if (report.errorCount > 0) {
    console.log(chalk.red(`  ${report.errorCount} error(s)`));
  }
  if (report.warningCount > 0) {
    console.log(chalk.yellow(`  ${report.warningCount} warning(s)`));
  }
  if (report.infoCount > 0) {
    console.log(chalk.blue(`  ${report.infoCount} info`));
  }
  if (fixMode && report.fixedCount > 0) {
    console.log(chalk.green(`  ${report.fixedCount} fixed`));
  }

  console.log();
}

function formatResult(result: LintResult): void {
  const severityIcon =
    result.severity === "error"
      ? chalk.red("x")
      : result.severity === "warning"
        ? chalk.yellow("!")
        : chalk.blue("i");

  const file = chalk.underline(result.path);
  const line = result.line ? `:${result.line}` : "";

  console.log(`\n${severityIcon} ${file}${line}`);
  console.log(`  ${result.message}`);

  if (result.suggestion) {
    console.log(chalk.dim(`  Suggestion: ${result.suggestion}`));
  }
}

program.parse();
