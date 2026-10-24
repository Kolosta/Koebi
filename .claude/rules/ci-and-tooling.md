---
paths:
  - ".github/**"
  - "xtask/**"
  - ".githooks/**"
  - "scripts/**"
---

# CI, automation and tooling

- Pin third-party actions by full commit SHA with the version in a comment
  (`uses: owner/action@<sha> # v1.2.3`); Dependabot keeps them updated.
- Least privilege: workflow-level `permissions: contents: read`; widen per job only.
- Never interpolate untrusted values (`github.head_ref`, PR titles, comment bodies) into `run:`
  scripts; pass them through `env:`. Never check out PR code in `pull_request_target`.
- The branch rulesets require exactly two checks: `CI result` and `PR policy result`. A new
  CI job must be added to the `needs` of `ci-result`; renaming those two jobs breaks merges.
- Install tools like a developer would: the Vulkan SDK version lives in
  `.github/vulkan-sdk-version` (single source of truth, also used for SPIR-V checks), the
  MoltenVK release for iOS in `.github/moltenvk-version`.
- Logic belongs in `xtask` (Rust, cross-platform, unit-tested), not in long YAML or shell
  scripts. xtask stays std-only.
- Keep `cargo xtask ci` and `.github/workflows/ci.yml` equivalent: what CI checks, developers
  can run locally.
- Workflows run `run:` steps through `.github/scripts/ci-step.sh` (error lines become
  annotations); jobs without a checkout set `defaults.run.shell: bash` themselves.
- Use `/ci-debug` to investigate a failing run before changing workflows. Without `gh` auth,
  read `check-runs/<job_id>/annotations` from the public API (logs need admin rights).
