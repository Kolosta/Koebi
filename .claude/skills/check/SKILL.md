---
name: check
description: Run the local CI gate (format, clippy, tests, docs, shaders, cargo-deny, typos) and report only the failures. Use before declaring any code task done and before opening a pull request.
argument-hint: "[--fast]"
context: fork
agent: general-purpose
background: false
allowed-tools: Bash(cargo *) PowerShell(cargo *) Bash(typos *) Read Grep Glob
---

Run `cargo xtask ci $ARGUMENTS` from the repository root and report the outcome.

- If everything passes, answer with one line: `CI gate: all checks passed` plus the summary
  table printed by xtask.
- If something fails, return for each failing check: the check name, the exact error lines
  (file:line, lint name, test name and assertion message), and the most likely cause. Quote
  at most ~30 lines per failure; never paste full logs.
- For GPU test failures, include any `[vulkan]` validation messages verbatim.
- Do not modify any file: this skill only diagnoses.
