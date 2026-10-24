//! Validation-layer messages and debug object names.

use std::ffi::{CStr, CString, c_void};
use std::sync::atomic::{AtomicU32, Ordering};

use ash::vk;

use crate::device::DeviceShared;

/// Counters shared with the debug messenger callback.
#[derive(Debug, Default)]
pub(crate) struct DebugState {
    pub(crate) errors: AtomicU32,
    pub(crate) warnings: AtomicU32,
}

/// A fresh messenger description pointing at `state`.
///
/// Always build a new value per use: `push_next` rewrites the `p_next` of chained structs.
pub(crate) fn messenger_create_info(
    state: &DebugState,
) -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback))
        .user_data(std::ptr::from_ref(state).cast_mut().cast::<c_void>())
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _types: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    user_data: *mut c_void,
) -> vk::Bool32 {
    // SAFETY: Vulkan passes null or a callback data structure valid for this call.
    let message = unsafe { data.as_ref() }
        .map(|data| data.p_message)
        .filter(|message| !message.is_null())
        // SAFETY: `p_message` is a NUL-terminated string valid for the duration of the call.
        .map_or(std::borrow::Cow::Borrowed("<no message>"), |message| unsafe {
            CStr::from_ptr(message).to_string_lossy()
        });
    // SAFETY: `user_data` is the `DebugState` owned by the instance, which destroys the
    // messenger before freeing the state.
    let state = unsafe { user_data.cast::<DebugState>().as_ref() };

    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        if let Some(state) = state {
            state.errors.fetch_add(1, Ordering::Relaxed);
        }
        koebi_core::error!("[vulkan] {message}");
    } else {
        if let Some(state) = state {
            state.warnings.fetch_add(1, Ordering::Relaxed);
        }
        koebi_core::warn!("[vulkan] {message}");
    }
    vk::FALSE
}

/// Attaches a human-readable name to a Vulkan object (shown by validation layers, RenderDoc,
/// Nsight...). No-op when `VK_EXT_debug_utils` is unavailable.
pub(crate) fn set_object_name<H: vk::Handle>(device: &DeviceShared, handle: H, name: &str) {
    let Some(debug_utils) = device.debug_utils.as_ref() else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let Ok(name) = CString::new(name) else { return };
    let info = vk::DebugUtilsObjectNameInfoEXT::default()
        .object_handle(handle)
        .object_name(&name);
    // SAFETY: `handle` is a live object of this device and `info` is valid for the call.
    if let Err(error) = unsafe { debug_utils.set_debug_utils_object_name(&info) } {
        koebi_core::debug!("could not name Vulkan object: {error}");
    }
}
