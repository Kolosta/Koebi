# Koebi documentation

## Architecture

- [Overview](architecture/overview.md): layers, crates, frame lifecycle, ownership.
- [GPU abstraction](architecture/gpu-abstraction.md): the Vulkan RHI, and why it differs from
  Blender's GPU module.
- [Architecture decision records](adr/README.md): the reasons behind every structural choice.

## Development

- [Setup](development/setup.md): toolchains per OS (Rust, Vulkan SDK, CMake, Android, Xcode).
- [Gitflow](development/gitflow.md): branches, commits, pull requests, releases.
- [Coding standards](development/coding-standards.md): Rust style, unsafe, errors, performance.
- [Testing](development/testing.md): test levels, GPU tests, golden images.
- [CI/CD](development/ci-cd.md): workflows, required checks, releases, artifacts.
- [Platforms](development/platforms.md): building and running on each platform.
- [GitHub setup](development/github-setup.md): branch protection, labels, security features.

## AI-assisted development

- [Working with Claude Code](ai/claude-code.md): the configuration in this repository, the
  available options (plugins, skills, agents, hooks, MCP, GitHub Actions) and token hygiene.
