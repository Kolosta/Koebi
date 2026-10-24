---
name: ci-debug
description: Diagnose a failing GitHub Actions run (CI, PR policy, Release) and propose the fix. Use when a pull request or push has red checks.
argument-hint: "[run-id | pr-number]"
context: fork
agent: general-purpose
background: false
allowed-tools: Bash(gh *) Bash(git *) Bash(curl *) Read Grep Glob WebFetch
---

Investigate the failing GitHub Actions run: $ARGUMENTS (default: the latest failed run of the
current branch).

1. Find the run: `gh run list --branch "$(git branch --show-current)" --limit 5` or
   `gh pr checks <pr>`.
2. Read only the failing steps: `gh run view <run-id> --log-failed`. Use
   `gh run view <run-id> --json jobs --jq '.jobs[] | select(.conclusion=="failure") | .name'`
   to list failing jobs. Start with the annotations: every failing `run:` step publishes its
   error lines through `.github/scripts/ci-step.sh`. Without `gh` authentication, read them
   from the public API: `https://api.github.com/repos/Kolosta/Koebi/actions/runs/<run-id>/jobs`
   then `https://api.github.com/repos/Kolosta/Koebi/check-runs/<job-id>/annotations` (logs
   require admin rights; 60 unauthenticated requests per hour).
3. Classify the failure: code (compile, clippy, test, golden image), environment (Vulkan SDK,
   lavapipe, MoltenVK, NDK, simulator), flaky infrastructure, or policy (branch name, PR title,
   cargo-deny allow-list).
4. For golden-image failures, note the uploaded `golden-failures-*` artifact
   (`gh run download <run-id> -n golden-failures-<OS>`).
5. Return: the failing job and step, the decisive log lines (≤ 30), the root cause, and a
   concrete fix (file and change) or the command to reproduce locally. Do not edit files.
