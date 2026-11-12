---
paths:
  - "crates/koebi-gpu/**/*.rs"
  - "crates/koebi-platform/**/*.rs"
  - "crates/koebi-testkit/src/alloc.rs"
  - "apps/koebi/src/lib.rs"
---

# Unsafe code and FFI

- Every `unsafe { }` block is preceded by `// SAFETY:` explaining why each precondition holds
  *here* (validity of handles, lifetimes, synchronization, alignment). Keep blocks minimal.
- Every `unsafe fn` and unsafe trait documents its contract under `# Safety`.
- Encapsulate: a safe public function must be sound for *any* input. If soundness depends on
  the caller, the function is `unsafe` (e.g. `Surface::from_raw`, `Instance::new`).
- Ownership of foreign handles is explicit: one Rust value owns each `Vk*`/`SDL_*` object and
  destroys it in `Drop`. Children hold an `Arc` to their parent so parents outlive them.
- Never let a panic cross an `extern "C"` boundary: wrap callback bodies in `catch_unwind`
  (see `koebi-platform/src/app.rs`).
- C strings: build `CString`s that outlive the call; reject interior NULs instead of
  truncating. Read returned C strings immediately (`CStr::from_ptr(..).to_string_lossy()`).
- `#[repr(C)]` structs uploaded to the GPU implement `koebi_gpu::Pod` with a `// SAFETY:`
  comment and a `const _: () = assert!(size_of::<T>() == N);` padding check.
- Outside `koebi-gpu`, `koebi-platform` and FFI glue, the only `unsafe` is the counting
  allocator of `koebi-testkit` (ADR 0026), installed by test binaries alone. Never install it
  in the application.
- `transmute` only between function pointers of identical signatures (loader symbols), with a
  `// SAFETY:` citing the specification.
- Edition 2024: `extern` blocks are `unsafe extern "C" { }`, exported symbols use
  `#[unsafe(no_mangle)]`.
