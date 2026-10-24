//! Koebi GPU: a thin, explicit render hardware interface (RHI) over Vulkan.
//!
//! Like Blender's `gpu` module, every pixel Koebi shows goes through this layer. Unlike it,
//! Koebi targets a single API (Vulkan, MoltenVK on Apple platforms), so there is no backend
//! trait and no dynamic dispatch: the abstraction exists for **safety and ergonomics**, not
//! portability between graphics APIs. See `docs/architecture/gpu-abstraction.md`.
//!
//! # Design rules
//!
//! - **Baseline**: Vulkan 1.2 with `VK_KHR_dynamic_rendering` and `VK_KHR_synchronization2`
//!   (core in 1.3). No render pass objects, no legacy barriers.
//! - **Ownership**: every object owns its Vulkan handle (RAII). Children keep their parents
//!   alive (`Arc`), so drop order can never destroy a device before its buffers.
//! - **Deferred destruction**: resources the GPU may still use are destroyed only after the
//!   submission that last used them completes. Dropping is always safe.
//! - **Explicit synchronization**: frames are paced by [`Presenter`]; blocking submissions
//!   ([`Device::submit_and_wait`]) are for uploads, readback and tests only.
//! - **Safe surface**: public functions are safe unless they take raw Vulkan handles. Raw
//!   handles are exposed (`handle()`, [`vk`]) as an escape hatch, never required.
//!
//! # Object graph
//!
//! ```text
//! VulkanLibrary ─► Instance ─► Adapter ─► Device ─► Buffer, Texture, ShaderModule, RenderPipeline
//!                     └──────► Surface ──────────► Presenter (swapchain + frames in flight)
//! ```

mod adapter;
mod buffer;
mod command;
mod debug;
mod device;
mod error;
mod format;
mod garbage;
mod instance;
pub mod loader;
mod memory;
mod pipeline;
mod pod;
mod presenter;
mod shader;
mod surface;
mod swapchain;
mod texture;

pub use adapter::{Adapter, AdapterInfo, AdapterKind, AdapterPreference};
/// Raw Vulkan bindings (`ash`), re-exported as an escape hatch.
pub use ash::vk;
pub use buffer::{Buffer, BufferDesc, BufferUsage};
pub use command::{CommandEncoder, LoadOp, RenderPass, RenderTarget, TextureState};
pub use device::Device;
pub use error::{Error, Result, version_string};
pub use format::{TextureFormat, VertexFormat};
pub use instance::{Instance, InstanceDesc, Validation, ValidationReport};
pub use loader::VulkanLibrary;
pub use memory::MemoryLocation;
pub use pipeline::{
    BlendMode, ColorTargetDesc, MAX_PUSH_CONSTANT_SIZE, PrimitiveTopology, RenderPipeline,
    RenderPipelineDesc, ShaderStage, VertexAttribute, VertexBufferLayout, VertexStepMode,
};
pub use pod::{Pod, bytes_of, cast_slice, cast_slice_mut};
pub use presenter::{FrameTarget, Presenter, PresenterDesc, RenderOutcome};
#[doc(hidden)]
pub use shader::__spirv_from_aligned;
pub use shader::{SPIRV_MAGIC, ShaderModule, spirv_words};
pub use surface::Surface;
pub use swapchain::PresentMode;
pub use texture::{Texture, TextureDesc, TextureUsage};

/// Number of frames the CPU may record ahead of the GPU.
///
/// Two keeps latency low (UI responsiveness) while letting CPU and GPU overlap.
pub const FRAMES_IN_FLIGHT: usize = 2;
