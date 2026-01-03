# lineup-agent

A high-performance rule-based linting and enforcement CLI tool built with Rust and Node.js.

## Overview

lineup-agent is a configurable linting engine that automatically checks and enforces project conventions. It features a Rust-powered core for blazing-fast performance, exposed to Node.js via native bindings.

## Features

- **Fast**: Native Rust engine with optimized release builds (LTO enabled)
- **Auto-fix**: Automatically fix issues where supported
- **Extensible**: Modular rule architecture for adding custom rules
- **Zero-config**: Runs all rules automatically with sensible defaults
- **Cross-platform**: Supports macOS (Intel/ARM), Linux (x64/ARM), and Windows

## Installation

```bash
npm install lineup-agent
# or
pnpm add lineup-agent
```

## Usage

### CLI

```bash
# Lint current directory
lineup-agent

# Lint a specific path
lineup-agent lint ./my-project

# Auto-fix issues
lineup-agent lint --fix

# Output as JSON
lineup-agent lint --json

# List available rules
lineup-agent rules
```

### Programmatic API

```typescript
import { lint, fix, createLineupAgent } from "lineup-agent";

// Quick lint
const report = lint("./my-project");
console.log(`Found ${report.errorCount} errors`);

// Quick fix
const fixReport = fix("./my-project");
console.log(`Fixed ${fixReport.fixedCount} issues`);

// Full engine control
const engine = createLineupAgent();
const results = engine.lint("./path");
const rules = engine.listRules();
```

## Architecture

```
lineup-agent/
├── src/                  # TypeScript source
│   ├── cli.ts           # CLI implementation
│   ├── index.ts         # Public API exports
│   └── native.ts        # Native bindings loader
├── native/              # Rust source
│   ├── src/
│   │   ├── lib.rs       # NAPI bindings
│   │   ├── engine.rs    # Rule execution engine
│   │   ├── types.rs     # Core types
│   │   └── rules/       # Rule implementations
│   └── Cargo.toml
└── package.json
```

The engine is built in Rust for performance and exposed to Node.js via [napi-rs](https://napi.rs/). Rules are implemented as Rust traits, enabling type-safe and efficient file system operations.

## Available Rules

### `claude-settings-hooks`

Ensures all git repositories have a properly configured `.claude/settings.json` with required hooks.

**What it checks:**

- Every folder with `.git` must have a `.claude/settings.json`
- Valid JSON syntax in `settings.json`
- `hooks` configuration object exists
- `PreToolUse` hooks are configured
- Bash matcher hook is present (prevents dangerous commands)

**Severity:** Error (can be auto-fixed)

**Auto-fix behavior:** Creates the `.claude` directory and `settings.json` with security hooks that block `git push --no-verify`.

## Development

### Prerequisites

- Node.js >= 18
- pnpm
- Rust toolchain (for building native modules)

### Setup

```bash
# Install dependencies
pnpm install

# Build everything (native + TypeScript)
pnpm build

# Development build (faster, unoptimized)
pnpm dev

# Run tests
pnpm test

# Type check
pnpm typecheck
```

### Adding New Rules

1. Create a new file in `native/src/rules/`
2. Implement the `Rule` trait
3. Register the rule in `RuleRegistry::register_builtin_rules()`
4. Rebuild with `pnpm build`

## Report Format

```typescript
interface LintReport {
  results: LintResult[];
  errorCount: number;
  warningCount: number;
  infoCount: number;
  fixedCount: number;
}

interface LintResult {
  ruleId: string;
  severity: "error" | "warning" | "info";
  message: string;
  path: string;
  line?: number;
  suggestion?: string;
}
```

## License

MIT
