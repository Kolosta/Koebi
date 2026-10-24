# Coding standards

These rules apply to humans and AI agents alike. Claude Code loads the same rules from
`.claude/rules/` when it edits matching files.

## Principles

1. **Performance by design**: no allocation in per-frame paths, no hidden GPU waits, data laid
   out for the GPU (instancing, batching).
2. **Safety at the boundary**: `unsafe` lives in `koebi-gpu`, `koebi-platform` and FFI glue,
   behind safe APIs that make undefined behaviour impossible.
3. **Explicit over clever**: small modules with one concept each, readable names, comments that
   explain *why*.
4. **Two runtime dependencies**: `ash` and `sdl3-sys` ([ADR 0003](../adr/0003-minimal-dependencies.md)).

## Layout and naming

- One concept per module; files stay under about 600 lines.
- Each module starts with a `//!` comment explaining its role; every public item has a doc
  comment.
- Types `UpperCamelCase`, functions `snake_case` verbs, booleans `is_*`/`has_*`,
  constructors `new`/`from_*`/`with_*`, fallible conversions `try_*`.
- No abbreviations except established ones (`vk`, `gpu`, `rgba`).
- Smallest visibility that works: `pub(crate)` for internals (`unreachable_pub` lint).

## Lints and formatting

- `rustfmt.toml` (max width 100) is applied by the pre-commit hook and, for Claude Code, after
  each edit.
- Clippy `all` and `pedantic` warn, and CI denies warnings. Workspace lints are in the root
  `Cargo.toml`.
- Fix warnings rather than silencing them. When an exception is justified, scope it tightly
  and give a reason: `#[allow(clippy::cast_possible_truncation, reason = "...")]`
  (`allow_attributes_without_reason` is enabled).

## Errors and panics

- Each library crate has one `#[non_exhaustive]` `Error` enum with a manual `Display` and a
  `Result<T>` alias. No `Box<dyn Error>` in library APIs.
- Add context at boundaries: `.ctx("vkCreateBuffer")` in `koebi-gpu`.
- `unwrap()` only in tests. `expect("...")` only for real invariants, with a message stating
  the invariant. Never panic on user or system input.
- Fallible public functions document `# Errors`; panicking ones `# Panics`.

## Unsafe code and FFI

- Every `unsafe` block is preceded by `// SAFETY:` explaining why each precondition holds
  there (clippy denies undocumented blocks).
- Every `unsafe fn` documents its contract under `# Safety`. A safe function must be sound for
  any input; otherwise it is `unsafe`.
- One Rust value owns each foreign handle and destroys it in `Drop`; children hold an `Arc` to
  their parent.
- Panics never cross `extern "C"` boundaries (`catch_unwind` in callbacks).
- GPU-visible structs are `#[repr(C)]`, implement `koebi_gpu::Pod`, and assert their size at
  compile time.
- Edition 2024 forms: `unsafe extern "C" { }`, `#[unsafe(no_mangle)]`.

## GPU code

- Vulkan 1.2 + dynamic rendering + synchronization2; 1.3/KHR entry points only through the
  `DeviceShared::cmd_*` dispatchers.
- Probe a feature in `Adapter::probe` before enabling it in `Device::new`.
- One RAII type per Vulkan object kind, destroyed through `DeviceShared::defer_destroy`.
- Name objects with `debug::set_object_name`; image transitions go through `TextureState`.
- Validation, including synchronization validation, stays silent.

Details: [gpu-abstraction.md](../architecture/gpu-abstraction.md).

## Rendering and shaders

- Scenes use logical pixels, origin top-left, y down; shaders apply the scale factor.
- Colors are `koebi_core::Color` (gamma-encoded sRGB, straight alpha), uploaded premultiplied;
  design tokens use OKLCH ([ADR 0004](../adr/0004-color-pipeline.md)).
- One primitive type means one instance struct, one pipeline and one GLSL pair, batched into
  instanced draws. Instance layouts must match the GLSL exactly.
- After editing GLSL, run `cargo xtask shaders` and commit the `.spv` files.

## Platform code

- SDL types never leave `koebi-platform`'s public API.
- SDL calls happen on the main thread inside `App` callbacks; never block there.
- Render on demand (`IterationRate::WaitEvent`); continuous rendering only for `--frames`.
- Exit codes come from `koebi_platform::exit_code`.

## Logging

Use `koebi_core::{error, warn, info, debug, trace}!`. Libraries never print to stdout or
stderr directly (clippy `print_stdout`/`print_stderr`).

## Performance checklist

- Per frame: no `Vec` growth after warm-up (`clear()` and reuse), no `clone()` of buffers, no
  Vulkan object creation, no `wait_idle` or `submit_and_wait`.
- Pass small `Copy` types by value; prefer slices to `Vec` parameters.
- Measure before optimizing; describe measurements in the pull request.

## Documentation and comments

- Doc examples must compile (doctests run in CI).
- Comments explain intent and invariants, not what the next line does.
- Architecture changes come with an ADR; behaviour changes update `docs/`.
