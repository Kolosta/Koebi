# Koebi documentation

## Reading the documentation

The documentation is plain Markdown, next to the code it describes. There is nothing to build
or serve.

- **On GitHub**: browse [`docs/`](https://github.com/Kolosta/Koebi/tree/develop/docs). GitHub
  renders the Markdown and the Mermaid diagrams.
- **In VS Code**: open a `.md` file and press `Ctrl+Shift+V` (preview) or `Ctrl+K V` (preview
  beside the source). Diagrams need the "Markdown Preview Mermaid Support" extension
  (`bierner.markdown-mermaid`).
- **API reference**: generated from the doc comments of every crate:

  ```sh
  cargo doc --workspace --no-deps --open
  ```

  The pages land in `target/doc/` and open in the browser. CI builds them with
  `-D warnings`, so broken links or missing docs fail the build.

Start with the [architecture overview](architecture/overview.md), then the guide for the area
you work on.

## Architecture

- [Overview](architecture/overview.md): layers, crates, frame lifecycle, ownership.
- [GPU abstraction](architecture/gpu-abstraction.md): the Vulkan RHI, and why it differs from
  Blender's GPU module.
- [Architecture decision records](adr/README.md): the reasons behind every structural choice.

## Design system

- [Design system](design-system/README.md): tokens, tiers, themes, overrides, views, color
  vision, and how to define and read tokens.
- [Components](design-system/components.md): how `koebi-ui` components read their tokens,
  and how to write one.
- [Gallery](design-system/gallery.md): what the application's window shows, where the design
  state is saved, and the design options of the command line.
- [Porting report](design-system/porting.md): the differences between Koebi's tokens and the
  token table of the previous application.

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
