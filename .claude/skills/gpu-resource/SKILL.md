---
name: gpu-resource
description: Add a new GPU resource or Vulkan object type to koebi-gpu (sampler, descriptor set layout, query pool, compute pipeline...) following the RHI's ownership, deferred-destruction and naming conventions.
argument-hint: "<resource name>"
paths: "crates/koebi-gpu/**"
---

Add the GPU resource: $ARGUMENTS

Follow the pattern of `crates/koebi-gpu/src/buffer.rs` and `texture.rs`:

1. **Description type** `FooDesc<'a>` with a `label: &'a str` and only the parameters Koebi
   needs (`Copy`, documented). Validate it before calling Vulkan and return
   `Error::InvalidArgument` with a clear message for bad input.
2. **Owning type** `Foo { device: Arc<DeviceShared>, raw: vk::Foo, .. }` created through a
   `Device::create_foo(&self, desc) -> Result<Foo>` method. On every error path after a
   successful `vkCreate*`, destroy what was created (see `Buffer::new`).
3. **Destruction**: if submitted command buffers can reference the object, add a
   `Garbage::Foo { .. }` variant (with its `destroy` arm) and call
   `self.device.defer_destroy(..)` in `Drop`. If they cannot (like shader modules), destroy
   immediately with a `// SAFETY:` comment saying why that is sound.
4. **Debug name**: `set_object_name(device, raw, desc.label)`.
5. **Capabilities**: if the resource needs a feature or extension, probe it in
   `Adapter::probe` and enable it in `Device::new` (1.2 + extension and 1.3 core paths).
6. **Public surface**: re-export from `lib.rs`, manual `Debug`, `handle()` escape hatch, docs
   with `# Errors`.
7. **Tests**: unit tests for pure logic (flag mapping, validation) and a headless integration
   test in `crates/koebi-gpu/tests/` that creates, uses and drops it with validation clean.
8. Update `docs/architecture/gpu-abstraction.md` if the object graph changes.
