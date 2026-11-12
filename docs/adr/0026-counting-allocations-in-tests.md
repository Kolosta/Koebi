# 0026. Counting allocations in tests with a global allocator of the test binaries

- Status: Accepted
- Date: 2026-11-12
- Deciders: @Kolosta

## Context

A steady frame allocates nothing (CLAUDE.md, *Hot paths*): scenes, caches and buffers keep their
storage from frame to frame. Until now, nothing checked this rule. Reviews caught some
allocations, but not hidden ones: a `format!` building a token path, a `Vec` collected to
iterate twice, a `String` cloned to satisfy the borrow checker. When the rule was first
measured ([#30](https://github.com/Kolosta/Koebi/issues/30)):

- shaping allocated on every line: kerning and ligatures walked the lookup lists into vectors;
- Settings › Theme allocated 270 times a frame, rebuilding the paths of 31 palettes;
- Settings › Keymaps allocated twice a frame, and the gallery 129 times.

Only an allocator sees every allocation, the hidden ones included. Rust lets a binary choose its
global allocator, but implementing `GlobalAlloc` is `unsafe`, and CLAUDE.md allows `unsafe`
only in `koebi-gpu`, `koebi-platform` and FFI glue.

## Decision

`koebi-testkit` provides a counting allocator, installed only by test binaries.

- **`CountingAllocator`** (`crates/koebi-testkit/src/alloc.rs`) forwards every call unchanged to
  `std::alloc::System` and counts allocations and reallocations per thread. The counter is a
  const-initialized thread-local `Cell`, so counting never allocates. Tests run in parallel, and
  each counts its own thread.
- **`allocations(f)`** returns how many allocations `f` made on the calling thread, with what it
  returned.
- **Test binaries only.** A crate that checks the rule installs the allocator under
  `#[cfg(test)]`:

  ```rust
  #[cfg(test)]
  #[global_allocator]
  static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;
  ```

  `koebi-testkit` is a dev-dependency only, so Koebi's executable keeps the system's allocator
  untouched.
- **The exception to the `unsafe` rule** covers this one module: an `unsafe impl GlobalAlloc`
  of four methods, each forwarding to `System`, each with its `// SAFETY:` comment.
- **What is asserted.** After warm-up frames, one more frame must count zero allocations:
  - every settings page, still and with the pointer over it;
  - the gallery;
  - a main window: bar, default workspace with an editor of each kind, status bar;
  - a paragraph drawn again, a line shaped again.

Frames that change something may allocate: an edit of the design, a menu opening, a resize.

## Consequences

- An allocation added to a steady frame fails a test that names the screen and the count. The
  one-off way to find it is a temporary backtrace in `count`. Keep it out of commits: capturing a
  backtrace allocates, so it needs a reentrancy guard.
- Code that reads the same derived data every frame finds it once and keeps it:
  - `PaletteTokens`: the tokens of a palette, from `Editable::tokens`;
  - the subtables of a font's kerning and ligatures, read when it loads;
  - the font families a dropdown lists.
- `koebi_ui::MenuItems` keeps the storage of a menu whose items borrow names known only at run
  time. It relies on `Vec`'s in-place collection, which reuses an emptied list's allocation for
  items of the same layout. A test checks that it allocates nothing.
- Counting costs one thread-local access per allocation in test binaries, nothing elsewhere.
- Allocations made by other threads (the system-font listing) are not counted: the frames are
  measured on the thread that builds them.

## Alternatives considered

- **Counting in the benchmark only** (`cargo xtask bench`, in release): too late, since the
  benchmark is not a CI check, and too coarse to name the screen.
- **A crate (`stats_alloc`, `dhat`)**: a dev-dependency for 60 lines, against the
  minimal-dependency policy ([ADR 0003](0003-minimal-dependencies.md)).
- **Reviews and `KOEBI_PROFILE`**: neither sees hidden allocations, which were most of those
  found.
