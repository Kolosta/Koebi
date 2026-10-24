---
paths:
  - "**/*.rs"
---

# Rust code style

- Structure: one concept per module; `//!` module doc first, then imports grouped std /
  external / crate (rustfmt orders them). Keep files under ~600 lines; split before that.
- Naming: types `UpperCamelCase`, functions `snake_case` verbs, booleans `is_`/`has_`,
  constructors `new`/`from_*`/`with_*`, fallible conversions `try_*`. No abbreviations except
  established ones (`vk`, `gpu`, `rgba`).
- Visibility: smallest that works. `pub(crate)` for internals (the `unreachable_pub` lint
  catches leaks). Expose raw handles only through explicit `handle()` escape hatches.
- Errors: each library crate has one `Error` enum (`#[non_exhaustive]`, manual `Display`) and a
  `Result<T>` alias. Add context at boundaries (`.ctx("vkCreateBuffer")` in koebi-gpu). No
  `Box<dyn Error>` in library APIs (xtask only).
- Panics: `unwrap()` only in tests; `expect()` with a message that states the invariant;
  `assert!` only for programming errors that must abort. Never `panic!` on user or system input.
- Casts: prefer `From`/`TryFrom`; an `as` cast that can truncate needs a local
  `#[allow(clippy::cast_possible_truncation, reason = "...")]` explaining why it cannot.
- Lints: fix warnings, do not silence them. Every `#[allow]` carries `reason = "..."`.
- Performance: no allocation in per-frame paths (reuse `Vec`s with `clear()`), pass small
  `Copy` types by value, avoid `clone()` of buffers, prefer slices over `Vec` parameters.
- Docs: every public item documented; fallible functions have `# Errors`, panicking ones
  `# Panics`, unsafe ones `# Safety`. Examples in docs must compile (doctests run in CI).
- Tests live next to the code (`#[cfg(test)] mod tests`) and are named
  `<subject>_<behaviour>` (e.g. `rect_contains_is_min_inclusive_max_exclusive`).
