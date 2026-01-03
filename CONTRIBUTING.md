# Contributing to lineup-agent

Thank you for your interest in contributing to lineup-agent! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and inclusive. We welcome contributors of all backgrounds and experience levels.

## Getting Started

### Prerequisites

- **Node.js** >= 18
- **pnpm** (recommended package manager)
- **Rust** toolchain (stable) - [Install Rust](https://rustup.rs/)
- **Git**

### Development Setup

```bash
# Clone the repository
git clone https://github.com/tupe12334/lineup-agent.git
cd lineup-agent

# Install dependencies
pnpm install

# Build the project
pnpm build

# Run tests
pnpm test
```

### Build Commands

| Command | Description |
|---------|-------------|
| `pnpm build` | Full production build (native + TypeScript) |
| `pnpm dev` | Development build (faster, debug mode) |
| `pnpm build:native` | Build only the Rust native module |
| `pnpm build:ts` | Build only the TypeScript code |
| `pnpm test` | Run test suite |
| `pnpm typecheck` | Run TypeScript type checking |

## Project Structure

```
lineup-agent/
├── src/                      # TypeScript source
│   ├── cli.ts               # CLI entry point
│   ├── index.ts             # Public API
│   └── native.ts            # Native module loader
├── native/                   # Rust source
│   ├── src/
│   │   ├── lib.rs           # NAPI bindings
│   │   ├── engine.rs        # Rule execution engine
│   │   ├── types.rs         # Shared types
│   │   └── rules/           # Rule implementations
│   │       ├── mod.rs       # Rule registry
│   │       └── *.rs         # Individual rules
│   └── Cargo.toml
├── dist/                     # Built output (generated)
└── package.json
```

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Create a new issue with:
   - Clear, descriptive title
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Node.js version, etc.)

### Suggesting Features

1. Open an issue describing:
   - The problem you're trying to solve
   - Your proposed solution
   - Alternative approaches considered

### Submitting Code

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Ensure tests pass: `pnpm test`
5. Ensure types check: `pnpm typecheck`
6. Commit with a descriptive message
7. Push and open a Pull Request

## Adding a New Rule

Rules are the core of lineup-agent. Here's how to add one:

### 1. Create the Rule File

Create a new file in `native/src/rules/`:

```rust
// native/src/rules/my_rule.rs

use crate::rules::{Rule, RuleError};
use crate::types::{LintResult, RuleContext, Severity};

pub struct MyRule;

impl MyRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MyRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for MyRule {
    fn id(&self) -> &'static str {
        "my-rule-id"
    }

    fn name(&self) -> &'static str {
        "My Rule Name"
    }

    fn description(&self) -> &'static str {
        "Description of what this rule checks"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Your checking logic here
        // Use context.root for the target path
        // Push LintResult items to results

        results
    }

    // Optional: implement auto-fix
    fn can_fix(&self) -> bool {
        false
    }

    fn fix(&self, _context: &RuleContext) -> Result<u32, RuleError> {
        Err(RuleError::FixNotSupported)
    }
}
```

### 2. Export the Module

Add to `native/src/rules/mod.rs`:

```rust
pub mod my_rule;
```

### 3. Register the Rule

In `native/src/rules/mod.rs`, add to `register_builtin_rules()`:

```rust
fn register_builtin_rules(&mut self) {
    self.register(Arc::new(claude_settings::ClaudeSettingsRule::new()));
    self.register(Arc::new(my_rule::MyRule::new()));  // Add this
}
```

### 4. Build and Test

```bash
pnpm build
lineup-agent rules  # Verify your rule appears
```

## Coding Standards

### Rust

- Run `cargo clippy` and address warnings
- Follow standard Rust naming conventions
- Use `thiserror` for error types
- Document public APIs with doc comments

### TypeScript

- Use TypeScript strict mode
- Prefer explicit types over inference for public APIs
- Use `interface` for object shapes
- Keep CLI output user-friendly

### Commits

- Use clear, descriptive commit messages
- Start with a verb: "Add", "Fix", "Update", "Remove"
- Reference issues when applicable: "Fix #123"

## Testing

### Running Tests

```bash
# Run all tests
pnpm test

# Run tests in watch mode
pnpm test -- --watch
```

### Writing Tests

- Place tests alongside source files or in a `__tests__` directory
- Test both success and failure cases
- For rules, test:
  - Detection of violations
  - Correct severity levels
  - Auto-fix behavior (if applicable)

## Pull Request Process

1. Update documentation if needed
2. Ensure all tests pass
3. Request review from maintainers
4. Address feedback promptly
5. Squash commits if requested

## Release Process

Releases are managed by maintainers:

1. Version bump in `package.json` and `Cargo.toml`
2. Update changelog
3. Create git tag
4. CI builds and publishes to npm

## Getting Help

- Open an issue for questions
- Check existing issues and discussions
- Review the README and this guide

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
