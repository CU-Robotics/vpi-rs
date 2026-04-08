use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ffi::c_void;
use std::marker::{PhantomData, PhantomPinned};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr;

pub struct ArrayParts {
    buffer_type: sys::VPIArrayBufferType,
    array_type: sys::VPIArrayType,
    data: *mut c_void,
    size: i32,
    capacity: i32,
    stride_bytes: i32,
    _pin: PhantomPinned,
}

impl ArrayParts {
    pub fn new(
        buffer_type: sys::VPIArrayBufferType,
        array_type: sys::VPIArrayType,
        data: *mut c_void,
        size: i32,
        capacity: i32,
        stride_bytes: i32,
    ) -> Pin<Box<Self>> {
        Box::pin(Self {
            buffer_type,
            array_type,
            size,
            capacity,
            stride_bytes,
            data,
            _pin: PhantomPinned,
        })
    }

    fn as_aos(self: Pin<&mut Self>) -> sys::VPIArrayBufferAOS {
        let this = unsafe { self.get_unchecked_mut() };

        sys::VPIArrayBufferAOS {
            type_: this.array_type,
            sizePointer: &raw mut this.size,
            capacity: this.capacity,
            strideBytes: this.stride_bytes,
            data: this.data,
        }
    }
}

pub struct BorrowedArray<'data> {
    inner: VpiArray,
    parts: ArrayParts,
    _marker: PhantomData<&'data mut ()>,
}

impl<'data> Deref for BorrowedArray<'data> {
    type Target = VpiArray;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'data> DerefMut for BorrowedArray<'data> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'data> BorrowedArray<'data> {
    /// The array view does not own the data and will not free it.
    pub unsafe fn from_raw_parts(
        buffer_type: sys::VPIArrayBufferType,
        parts: ArrayParts,
    ) -> VpiResult<Self> {
        let array_ptr = ptr::null_mut();

        Ok(Self {
            inner: VpiArray {
                handle: ptr::NonNull::new(array_ptr).expect(util::FFI_SUCCESS_CONTRACT),
            },
            parts: parts,
            _marker: PhantomData,
        })
    }
}

pub struct ArrayLock<'arr> {
    /**
     * Either a view over a host buffer or a CUDA buffer.
     */
    array: &'arr mut VpiArray,
    data: sys::VPIArrayData,
}

impl<'arr> ArrayLock<'arr> {
    pub fn len(&self) -> usize {
        unsafe { *self.data.buffer.aos.sizePointer as usize }
    }

    pub fn size_bytes(&self) -> usize {
        let type_size = unsafe { sys::vpiArrayTypeGetSize(self.data.buffer.aos.type_) as usize };

        self.len() * type_size
    }

    /// This could be a pointer to host or device memory!
    pub fn as_ptr(&self) -> *mut c_void {
        self.data.buffer.aos.data
    }
}

impl Drop for ArrayLock<'_> {
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
     * - ArrayLock
     *  ^ Locking pretty much useless without data
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

    fn unlock(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiArrayUnlock(self.handle.as_ptr()))? };
        Ok(())
    }

    pub fn lock<'arr>(
        &'arr mut self,
        mode: sys::VPILockMode,
        buffer_type: sys::VPIArrayBufferType,
    ) -> VpiResult<ArrayLock<'arr>> {
        let mut data = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiArrayLockData(
                self.handle.as_ptr(),
                mode,
                buffer_type,
                &raw mut data,
            ))?
        };

        Ok(ArrayLock { array: self, data })
    }
}

impl Drop for VpiArray {
    fn drop(&mut self) {
        unsafe {
            sys::vpiArrayDestroy(self.handle.as_ptr());
        }
    }
}
