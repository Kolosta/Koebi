//! SPIR-V shader modules.
//!
//! Koebi compiles GLSL to SPIR-V ahead of time (`cargo xtask shaders`) and embeds the binaries
//! with [`include_spirv!`](crate::include_spirv): no shader compiler ships with the app.

use std::fmt;
use std::sync::Arc;

use ash::vk;

use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::{Error, Result};

/// First word of every SPIR-V module.
pub const SPIRV_MAGIC: u32 = 0x0723_0203;

/// A compiled shader module.
pub struct ShaderModule {
    device: Arc<DeviceShared>,
    raw: vk::ShaderModule,
}

impl ShaderModule {
    pub(crate) fn new(device: &Arc<DeviceShared>, label: &str, spirv: &[u32]) -> Result<Self> {
        if spirv.first() != Some(&SPIRV_MAGIC) {
            return Err(Error::InvalidArgument(format!(
                "shader {label:?} is not SPIR-V"
            )));
        }
        let info = vk::ShaderModuleCreateInfo::default().code(spirv);
        // SAFETY: valid device; `spirv` is a complete SPIR-V module (magic checked).
        let raw =
            unsafe { device.raw.create_shader_module(&info, None) }.ctx("vkCreateShaderModule")?;
        set_object_name(device, raw, label);
        Ok(Self {
            device: Arc::clone(device),
            raw,
        })
    }

    /// The raw module handle.
    pub fn handle(&self) -> vk::ShaderModule {
        self.raw
    }
}

impl fmt::Debug for ShaderModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ShaderModule").field(&self.raw).finish()
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        // SAFETY: shader modules are only read during pipeline creation (pipelines keep their
        // own copy of the code), never by submitted command buffers, so immediate destruction
        // is safe.
        unsafe { self.device.raw.destroy_shader_module(self.raw, None) };
    }
}

/// Converts SPIR-V bytes (e.g. read from disk) to words, fixing endianness if needed.
///
/// # Errors
///
/// Returns [`Error::InvalidArgument`] if the length is not a multiple of four or the magic
/// number is missing.
pub fn spirv_words(bytes: &[u8]) -> Result<Vec<u32>> {
    if !bytes.len().is_multiple_of(4) || bytes.len() < 20 {
        return Err(Error::InvalidArgument(format!(
            "{} bytes is not SPIR-V",
            bytes.len()
        )));
    }
    let (chunks, _) = bytes.as_chunks::<4>();
    let words: Vec<u32> = chunks
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk))
        .collect();
    match words[0] {
        SPIRV_MAGIC => Ok(words),
        swapped if swapped.swap_bytes() == SPIRV_MAGIC => {
            Ok(words.into_iter().map(u32::swap_bytes).collect())
        }
        _ => Err(Error::InvalidArgument("missing SPIR-V magic number".into())),
    }
}

/// Embeds a SPIR-V binary as a 4-byte-aligned `&'static [u32]`, without runtime copies.
///
/// ```ignore
/// static RECT_VERT: &[u32] = koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/rect.vert.spv"));
/// ```
#[macro_export]
macro_rules! include_spirv {
    ($bytes:expr) => {{
        #[repr(C, align(4))]
        struct Aligned<T: ?Sized>(T);
        static ALIGNED: &Aligned<[u8]> = &Aligned(*$bytes);
        // SAFETY: `Aligned` is `align(4)`, so its byte contents are 4-byte aligned.
        unsafe { $crate::__spirv_from_aligned(&ALIGNED.0) }
    }};
}

/// Implementation detail of [`include_spirv!`](crate::include_spirv).
///
/// # Safety
///
/// `bytes` must be 4-byte aligned.
///
/// # Panics
///
/// Panics (at compile time when used in a `static`) if the length is not a multiple of four.
#[doc(hidden)]
pub const unsafe fn __spirv_from_aligned(bytes: &'static [u8]) -> &'static [u32] {
    assert!(
        bytes.len().is_multiple_of(4),
        "SPIR-V data must be a multiple of 4 bytes"
    );
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "alignment is part of the safety contract"
    )]
    let words = bytes.as_ptr().cast::<u32>();
    // SAFETY: the caller guarantees alignment, the length was checked above, `u32` accepts any
    // bit pattern and the data is `'static`.
    unsafe { core::slice::from_raw_parts(words, bytes.len() / 4) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_bytes(magic: u32) -> Vec<u8> {
        let mut bytes = magic.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 16]);
        bytes
    }

    #[test]
    fn spirv_words_accepts_both_endiannesses() {
        assert_eq!(
            spirv_words(&module_bytes(SPIRV_MAGIC)).unwrap()[0],
            SPIRV_MAGIC
        );
        assert_eq!(
            spirv_words(&module_bytes(SPIRV_MAGIC.swap_bytes())).unwrap()[0],
            SPIRV_MAGIC
        );
    }

    #[test]
    fn spirv_words_rejects_garbage() {
        assert!(spirv_words(&[1, 2, 3]).is_err());
        assert!(spirv_words(&module_bytes(0xDEAD_BEEF)).is_err());
    }

    #[test]
    fn include_spirv_produces_aligned_words() {
        static WORDS: &[u32] = include_spirv!(&[0x03_u8, 0x02, 0x23, 0x07, 1, 0, 0, 0]);
        assert_eq!(WORDS, &[SPIRV_MAGIC, 1]);
    }
}
