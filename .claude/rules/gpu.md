---
paths:
  - "crates/koebi-gpu/**"
---

# koebi-gpu (Vulkan RHI)

Read `docs/architecture/gpu-abstraction.md` before changing structure.

- Baseline: Vulkan 1.2 + `VK_KHR_dynamic_rendering` + `VK_KHR_synchronization2` (core in
  1.3). Call 1.3/KHR entry points only through `DeviceShared::cmd_*` dispatchers so both paths
  keep working. Probe any new feature/extension in `Adapter::probe` and enable it in
  `Device::new`; never assume it.
- No render pass or framebuffer objects; only dynamic rendering. Only synchronization2
  barriers (`ImageMemoryBarrier2`, `vkQueueSubmit2`).
- Resources: one RAII type per Vulkan object kind. Dropping must call
  `DeviceShared::defer_destroy(Garbage::...)` unless the object is never referenced by
  submitted work (shader modules). Add a `Garbage` variant for each new kind.
- Name every object with `debug::set_object_name` (label from the `*Desc`).
- New state transitions go through `TextureState` (stage, access, layout). The swapchain
  acquire semaphore is waited at `COLOR_ATTACHMENT_OUTPUT`; keep barrier source stages chained.
- Queue access is externally synchronized: submit and present only via `DeviceShared::submit`
  and `present` (they take the queue mutex).
- Never add `vkDeviceWaitIdle`/fence waits in per-frame code. `submit_and_wait` is for
  uploads, readback and tests.
- Errors: map `VkResult` with `.ctx("vkFunctionName")`; device and surface loss have their own
  `Error` variants that callers handle.
- Every change keeps validation (including synchronization validation) silent: run the GPU
  tests with `KOEBI_VALIDATION=required KOEBI_REQUIRE_GPU=1 cargo nextest run -p koebi-gpu`.
