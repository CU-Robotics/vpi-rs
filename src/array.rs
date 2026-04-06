use crate::sys::{self};
use crate::util::{self, VpiResult, check};
use std::ffi::c_void;
use std::ptr;

pub struct WrappedArray {
    pub array: VpiArray,
}

impl WrappedArray {}

/// Safe counterpart to C struct
pub struct AosView<'arr> {
    pub array_type: sys::VPIArrayType,
    pub capacity: usize,
    pub stride: usize,

    data_handle: ptr::NonNull<u8>,
    len: &'arr mut i32,
}

impl<'arr> AosView<'arr> {
    pub fn as_slice<T>(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.data_handle.cast::<T>().as_ptr(), *self.len as usize)
        }
    }

    pub fn as_mut_slice<T>(&mut self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.data_handle.cast::<T>().as_ptr(),
                *self.len as usize,
            )
        }
    }
}

pub struct VpiArrayLock<'arr> {
    array: &'arr mut VpiArray,
    data: sys::VPIArrayData,
}

impl<'arr> VpiArrayLock<'arr> {
    pub fn aos_view(&'arr mut self) -> AosView<'arr> {
        unsafe {
            let aos = self.data.buffer.aos;

            AosView {
                array_type: aos.type_,
                data_handle: ptr::NonNull::new(aos.data.cast::<u8>())
                    .expect(util::FFI_SUCCESS_CONTRACT),
                len: &mut *aos.sizePointer,
                capacity: aos.capacity as usize,
                stride: aos.strideBytes as usize,
            }
        }
    }
}

impl Drop for VpiArrayLock<'_> {
    fn drop(&mut self) {
        // Could throw an error but no way to handle it in drop
        let _ = self.array.unlock();
    }
}

pub struct VpiArray {
    pub(crate) handle: ptr::NonNull<sys::VPIArrayImpl>,
}

impl VpiArray {
    /**
     * Skip:
     * - vpiArraySetWrapper
     *  ^ Ownership semantics are complicated
     */
    pub fn new(capacity: usize, array_type: sys::VPIArrayType, flags: u64) -> VpiResult<Self> {
        let mut array_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiArrayCreate(
                capacity as i32,
                array_type,
                flags,
                &raw mut array_ptr,
            ))?
        };

        Ok(Self {
            handle: ptr::NonNull::new(array_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    pub fn get_size(&self) -> VpiResult<usize> {
        let mut size = i32::default();

        unsafe { check(sys::vpiArrayGetSize(self.handle.as_ptr(), &raw mut size))? };

        Ok(size as usize)
    }

    pub fn set_size(&self, size: usize) -> VpiResult<()> {
        unsafe { check(sys::vpiArraySetSize(self.handle.as_ptr(), size as i32))? };
        Ok(())
    }

    pub fn get_capacity(&self) -> VpiResult<usize> {
        let mut capacity = i32::default();

        unsafe {
            check(sys::vpiArrayGetCapacity(
                self.handle.as_ptr(),
                &raw mut capacity,
            ))?
        };

        Ok(capacity as usize)
    }

    pub fn get_stride_bytes(&self) -> VpiResult<usize> {
        let mut stride = i32::default();

        unsafe {
            check(sys::vpiArrayGetStrideBytes(
                self.handle.as_ptr(),
                &raw mut stride,
            ))?
        };

        Ok(stride as usize)
    }

    pub fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe { check(sys::vpiArrayGetFlags(self.handle.as_ptr(), &raw mut flags))? };

        Ok(flags)
    }

    pub fn get_type(&self) -> VpiResult<sys::VPIArrayType> {
        let mut array_type = sys::VPIArrayType::default();

        unsafe {
            check(sys::vpiArrayGetType(
                self.handle.as_ptr(),
                &raw mut array_type,
            ))?
        };

        Ok(array_type)
    }

    // fn lock(&self, mode: sys::VPILockMode) -> VpiResult<()> {
    //     unsafe { check(sys::vpiArrayLock(self.handle.as_ptr(), mode))? };
    //     Ok(())
    // }

    fn unlock(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiArrayUnlock(self.handle.as_ptr()))? };
        Ok(())
    }

    pub fn lock_and_get_data<'arr>(
        &'arr mut self,
        mode: sys::VPILockMode,
        buffer_type: sys::VPIArrayBufferType,
    ) -> VpiResult<VpiArrayLock<'arr>> {
        let mut data = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiArrayLockData(
                self.handle.as_ptr(),
                mode,
                buffer_type,
                &raw mut data,
            ))?
        };

        Ok(VpiArrayLock { array: self, data })
    }
}

impl Drop for VpiArray {
    fn drop(&mut self) {
        unsafe {
            sys::vpiArrayDestroy(self.handle.as_ptr());
        }
    }
}
