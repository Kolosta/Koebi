# Changelog

All notable changes to Koebi are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workspace foundation: `koebi-core` (geometry, color science, logging), `koebi-gpu` (Vulkan
  render hardware interface), `koebi-render` (instanced 2D renderer), `koebi-platform` (SDL3
  layer), `koebi-testkit` (headless GPU tests, golden images) and the `koebi` application.
- A window rendering an anti-aliased rounded rectangle with a border, on Windows, Linux, macOS,
  Android and iOS.
- Development automation (`cargo xtask`), CI/CD on GitHub Actions, gitflow tooling and the
  Claude Code configuration.
