# CI/CD

All workflows live in `.github/workflows/`. Third-party actions are pinned by commit SHA and
updated by Dependabot. The Vulkan SDK version comes from `.github/vulkan-sdk-version`.

## Workflows

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | pushes to `main`/`develop`, pull requests into them | quality gate on every platform |
| `pr-policy.yml` | pull request events, pushes to `main` | gitflow branch rules, Conventional PR title, release version check |
| `package.yml` | called by CI, Nightly and Release | release builds of every platform |
| `release.yml` | pushes to `main` | tag, GitHub Release, back-merge pull request |
| `nightly.yml` | every night, manual | security advisories, Miri, fresh packages |
| `labeler.yml` | pull requests | area labels from changed paths |
| `claude.yml` | `@claude` mentions (opt-in) | Claude Code answers or implements on GitHub |
| `claude-code-review.yml` | pull requests (opt-in) | automated Claude Code review |

## CI jobs

| Job | Runs on | What |
|---|---|---|
| Detect changes | Linux | skips heavy jobs for pull requests touching only docs or AI config |
| Format | Linux | `cargo fmt --check` |
| Lint | Linux | clippy pedantic `-D warnings`, rustdoc `-D warnings` |
| Dependencies | Linux | `cargo deny check` (licenses, bans, advisories, sources) |
| Spelling | Linux | typos |
| Shaders | Linux | committed SPIR-V matches the GLSL (`cargo xtask shaders --check`) |
| Test (Linux, Windows, macOS) | each OS | build, all tests with validation required, doctests |
| Android | Linux + emulator | Android clippy, APK, test binaries and APK smoke test on an emulator |
| iOS | macOS + simulator | iOS clippy, device build, tests and app smoke test in a simulator |
| Package | all | release artifacts (pushes only) |
| CI result | Linux | aggregates every job: the required check |

GPU drivers in CI: Mesa lavapipe on Linux (Ubuntu package) and Windows (mesa-dist-win), MoltenVK
on macOS. `KOEBI_REQUIRE_GPU=1` turns any GPU skip into a failure. Two details matter:

- The Vulkan loader ignores `VK_DRIVER_FILES` in elevated processes, and Windows runners run as
  administrator, so `.github/actions/setup-vulkan` also registers lavapipe under
  `HKLM\SOFTWARE\Khronos\Vulkan\Drivers`.
- Jobs that need a device (`device: "true"`) end the setup with `vulkaninfo --summary` and fail
  early, with its output as an annotation, when no device is visible.

The same lavapipe build runs locally for debugging: extract mesa-dist-win and point
`VK_DRIVER_FILES` at its `lvp_icd.x86_64.json` (Windows) before running the tests.

Every `run:` step goes through `.github/scripts/ci-step.sh` (`defaults.run.shell`): it behaves
like GitHub's `bash -eo pipefail` and, when the step fails, publishes the relevant error lines
(compiler errors, panics, CMake errors, test failures) as an annotation. Failing tests are also
annotated from the JUnit report. Most failures can be understood from the run summary without
opening the logs. Golden-image differences are uploaded as `golden-failures-<OS>` artifacts.

## Required checks

The rulesets require exactly two checks:

- `CI result` (from `ci.yml`), green when every CI job succeeded or was skipped on purpose;
- `PR policy result` (from `pr-policy.yml`).

A new CI job must be added to the `needs` list of `ci-result`. Renaming these two jobs breaks
merging until the rulesets are updated.

## Delivery

Before the merge, the PR policy's **Release version** job runs `cargo xtask release-check` on
`release/*` and `hotfix/*` pull requests: version, changelog section, unused tag, and a bump
at least as large as the Conventional Commits require ([gitflow.md](gitflow.md#versioning)).

Every push to `main` runs `release.yml`:

1. **Plan**: reads the workspace version; if tag `vX.Y.Z` exists, stops.
2. **Package**: builds Windows (zip), Linux (tar.gz), macOS (universal `Koebi.app`, zip),
   Android (release APK) and iOS (simulator app, unsigned device app).
3. **Publish**: generates notes with git-cliff, creates the annotated tag `vX.Y.Z` as
   `github-actions[bot]`, creates the GitHub Release with all artifacts, and opens the
   back-merge pull request into develop.

The back-merge pull request is opened with `GITHUB_TOKEN`, which triggers no workflow. Its
required checks come from `main`'s commit: CI runs on every push to `main`, and so does the
`PR policy result` job of `pr-policy.yml`.

Pushes to `develop` also build packages (14-day artifacts), so a broken release build is
caught before a release branch exists.

### Signing

- Android: without secrets, the release APK is signed with the debug key (installable, not
  publishable). For store builds, add repository secrets `KOEBI_ANDROID_KEYSTORE_BASE64`,
  `KOEBI_ANDROID_KEYSTORE_PASSWORD`, `KOEBI_ANDROID_KEY_ALIAS`, `KOEBI_ANDROID_KEY_PASSWORD`.
- iOS: device builds are ad-hoc signed; re-sign them with a distribution identity and a
  provisioning profile to install or publish them.
- macOS: bundles are ad-hoc signed; notarization needs an Apple Developer ID (future work).

## Debugging a failing run

```sh
gh run list --branch develop --limit 5
gh run view <run-id> --log-failed
```

In Claude Code, `/ci-debug` does this in a forked context and returns a diagnosis. Reproduce
locally with `cargo xtask ci`, or with the platform commands in [platforms.md](platforms.md).

## Security of workflows

- Workflow-level `permissions: contents: read`; jobs widen only what they need.
- Untrusted values (branch names, PR titles, comments) reach scripts through `env:`, never
  through `${{ }}` interpolation inside `run:`.
- `pull_request_target` is used only by the labeler, which never checks out pull request code.
