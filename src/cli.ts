#!/usr/bin/env node

import { Command } from "commander";
import { native } from "./native.js";
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";
import chalk from "chalk";
import type { LintReport, LintResult } from "./native.js";

const program = new Command();

program
  .name("lineup-agent")
  .description("Rule-based linting and enforcement CLI tool")
  .version("0.1.0");

program
  .command("lint")
  .description("Run linting rules on the specified path")
  .argument("[path]", "Path to lint", ".")
  .option("-c, --config <path>", "Path to config file")
  .option("--fix", "Automatically fix issues where possible")
  .option("--json", "Output results as JSON")
  .action((targetPath: string, options: LintOptions) => {
    const config = loadConfig(options.config);
    const engine = native.createEngine(JSON.stringify(config));

    const absolutePath = resolve(targetPath);
    const report: LintReport = options.fix
      ? engine.fix(absolutePath)
      : engine.lint(absolutePath);

    if (options.json) {
      console.log(JSON.stringify(report, null, 2));
    } else {
      formatReport(report, options.fix);
    }

    // Exit with error code if there are errors
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

program
  .command("init")
  .description("Initialize a lineup configuration file")
  .action(() => {
    const defaultConfig = {
      rules: {
        "claude-settings-hooks": {
          enabled: true,
          severity: "error",
          options: {},
        },
      },
    };

    console.log(JSON.stringify(defaultConfig, null, 2));
    console.log("\nCopy the above to lineup.config.json");
  });

interface LintOptions {
  config?: string;
  fix?: boolean;
  json?: boolean;
}

interface ConfigFile {
  rules?: Record<
    string,
    {
      enabled?: boolean;
      severity?: string;
      options?: Record<string, unknown>;
    }
  >;
}

function loadConfig(configPath?: string): ConfigFile {
  const paths = configPath
    ? [configPath]
    : ["lineup.config.json", ".lineuprc.json", ".lineuprc"];

  for (const p of paths) {
    const fullPath = resolve(p);
    if (existsSync(fullPath)) {
      try {
        return JSON.parse(readFileSync(fullPath, "utf-8"));
      } catch (e) {
        console.error(chalk.red(`Error reading config file ${fullPath}:`), e);
        process.exit(1);
      }
    }
  }

  return {}; // Default empty config
}

function formatReport(report: LintReport, fixMode?: boolean): void {
  if (report.results.length === 0) {
    console.log(chalk.green("\n  No issues found!\n"));
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
