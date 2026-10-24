//! Plain-old-data marker used for every CPU→GPU copy.

/// Types that can be copied to and from GPU memory byte for byte.
///
/// # Safety
///
/// Implementors must guarantee that:
/// - the type is `#[repr(C)]` (or a primitive/array) and contains **no padding bytes**, so
///   viewing it as `&[u8]` never exposes uninitialized memory;
/// - it contains no pointers, references or other handles whose meaning is process-local;
/// - **every bit pattern is a valid value**, so bytes read back from the GPU are sound.
///
/// Add `const _: () = assert!(size_of::<T>() == <sum of field sizes>);` next to each
/// implementation to catch accidental padding at compile time.
pub unsafe trait Pod: Copy + 'static {}

macro_rules! impl_pod {
    ($($ty:ty),* $(,)?) => {
        $(
            // SAFETY: primitive numeric types have no padding and accept every bit pattern.
            unsafe impl Pod for $ty {}
        )*
    };
}

impl_pod!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

// SAFETY: arrays of `Pod` values are contiguous with no padding between elements.
unsafe impl<T: Pod, const N: usize> Pod for [T; N] {}

/// Views a slice of `Pod` values as bytes.
pub fn cast_slice<T: Pod>(values: &[T]) -> &[u8] {
    // SAFETY: `T: Pod` guarantees there are no padding (uninitialized) bytes; `u8` has an
    // alignment of 1, and the length is the exact byte size of the slice.
    unsafe { core::slice::from_raw_parts(values.as_ptr().cast::<u8>(), size_of_val(values)) }
}

/// Views a slice of `Pod` values as mutable bytes.
pub fn cast_slice_mut<T: Pod>(values: &mut [T]) -> &mut [u8] {
    // SAFETY: as in `cast_slice`; additionally `T: Pod` accepts every bit pattern, so any byte
    // written through the returned slice leaves the values valid.
    unsafe {
        core::slice::from_raw_parts_mut(values.as_mut_ptr().cast::<u8>(), size_of_val(values))
    }
}

/// Views one `Pod` value as bytes.
pub fn bytes_of<T: Pod>(value: &T) -> &[u8] {
    cast_slice(core::slice::from_ref(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_slice_exposes_native_endian_bytes() {
        let values = [0x0403_0201_u32, 0x0807_0605];
        let bytes = cast_slice(&values);
        assert_eq!(bytes.len(), 8);
        assert_eq!(&bytes[..4], &0x0403_0201_u32.to_ne_bytes());
    }

    #[test]
    fn cast_slice_mut_writes_through() {
        let mut values = [0_u16; 2];
        cast_slice_mut(&mut values).copy_from_slice(&[1, 0, 2, 0]);
        assert_eq!(
            values,
            [u16::from_ne_bytes([1, 0]), u16::from_ne_bytes([2, 0])]
        );
    }

    #[test]
    fn bytes_of_arrays_covers_every_element() {
        let value = [1.0_f32, 2.0, 3.0, 4.0];
        assert_eq!(bytes_of(&value).len(), 16);
    }
}
