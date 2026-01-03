# pnpm-scripts false positive bug

## Bug Description

The `pnpm-usage` rule incorrectly flags scripts using `pnpm` commands as using `npm`.

## Error

```
! /Users/ofek/dev/git/github/tupe12334/lineup-agent/package.json
  Script 'build' uses npm command - consider using pnpm
  Suggestion: Replace 'npm' with 'pnpm' in script commands
```

## Root Cause

The check `script_cmd.contains("npm ")` returns `true` for strings like `"pnpm build"` because starting at index 1 of "pnpm ", you have "npm " (the 'n', 'p', 'm' from pnpm followed by the space).

## Fix

Added `contains_standalone_command()` helper function that uses proper word boundary detection to ensure "npm" is matched as a standalone command, not as a substring of "pnpm".

## Status: FIXED

Changes made to `native/src/rules/pnpm_usage.rs`:
- Added `contains_standalone_command()` helper with word boundary detection
- Added regression tests to prevent this issue from recurring
