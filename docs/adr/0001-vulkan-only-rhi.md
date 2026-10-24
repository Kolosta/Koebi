# 0001. Vulkan-only render hardware interface

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Koebi draws its whole interface itself: widgets, canvases, compositing. It needs precise
control over memory, synchronization, color handling and frame pacing, on Windows, Linux,
macOS, Android and iOS, with as few dependencies as possible.

Blender solves a similar problem with its `gpu` module: one in-house layer, with OpenGL, Vulkan
and Metal backends behind virtual interfaces. That design exists mainly to support several
graphics APIs.

## Decision

Koebi has a single in-house render hardware interface, `koebi-gpu`, written directly on Vulkan
through the `ash` bindings, with no backend abstraction.

- Baseline: Vulkan 1.2 with `VK_KHR_dynamic_rendering` and `VK_KHR_synchronization2` (core in
  1.3, used through the core or KHR entry points depending on the driver).
- Apple platforms use MoltenVK (`VK_KHR_portability_subset` handled at device creation).
- Every Vulkan object is a Rust value destroyed in `Drop`; objects the GPU may still use go
  through a deferred-destruction queue keyed by submission index.
- Dispatch is static: no trait objects, no virtual calls.
- Higher layers (`koebi-render`, the app) use `koebi-gpu` types only, never `vk` directly.

## Consequences

- One API to learn, test and optimize; modern explicit model (no render pass objects, explicit
  barriers) without legacy constraints.
- The quality of MoltenVK matters on macOS and iOS; the portability subset forbids a few
  features (for example triangle fans), which the renderer must not use.
- Vulkan's verbosity stays inside `koebi-gpu`; the rest of the code sees a small safe API.
- If MoltenVK ever blocks us, a Metal backend can be added under the same API, because no
  crate above `koebi-gpu` depends on Vulkan types.

## Alternatives considered

- **Several backends like Blender (Vulkan, Metal, Direct3D 12)**: triples the code and the test
  matrix for no platform gain, and forces a lowest-common-denominator API.
- **wgpu**: excellent and portable, but a large dependency tree, its own shading language and
  abstraction, and less control over memory and synchronization.
- **SDL3 GPU API**: already inside SDL, but it needs one shader format per backend and hides
  Vulkan features we plan to use (descriptor indexing, timestamp queries, HDR color spaces).
- **OpenGL / OpenGL ES**: deprecated on Apple platforms, weak multithreading, no future.
- **vulkano**: safe but heavy and opinionated, with long compile times and runtime checks we
  do not need.
