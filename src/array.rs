use crate::sys;
use crate::util::{self, VpiError, VpiResult, check};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr;

pub struct ArrayDataWrapper {
    pub data: sys::VPIArrayData,
    pub _size: Box<i32>,
}

pub trait ArrayDataBacking {
    fn array_data(&self) -> VpiResult<ArrayDataWrapper>;
}

#[derive(Default)]
pub struct ArrayDataBuilder {
    buffer_type: Option<sys::VPIArrayBufferType>,
    ptr: Option<*mut c_void>,
    size: Option<usize>,
    capacity: Option<usize>,
    stride_bytes: Option<usize>,
    item_type: Option<sys::VPIArrayType>,
}

impl ArrayDataBuilder {
    pub unsafe fn cuda(mut self) -> Self {
        self.buffer_type = Some(sys::VPIArrayBufferType_VPI_ARRAY_BUFFER_CUDA_AOS);
        self
    }

    pub unsafe fn ptr(mut self, ptr: *mut c_void) -> Self {
        self.ptr = Some(ptr);
        self
    }

    pub unsafe fn size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }

    pub unsafe fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    pub unsafe fn stride_bytes(mut self, stride_bytes: usize) -> Self {
        self.stride_bytes = Some(stride_bytes);
        self
    }

    pub unsafe fn item_type(mut self, item_type: sys::VPIArrayType) -> Self {
        self.item_type = Some(item_type);
        self
    }
}

impl ArrayDataBacking for ArrayDataBuilder {
    fn array_data(&self) -> VpiResult<ArrayDataWrapper> {
        // Validate all builder arguments
        let data_ptr = self
            .ptr
            .filter(|&p| !p.is_null())
            .ok_or(VpiError::App("Invalid data pointer given to array builder"))?;

        let size = self
            .size
            .filter(|&s| s > 0usize)
            .ok_or(VpiError::App("Invalid size given to array builder"))?;

        let capacity = self
            .capacity
            .or(Some(size))
            .filter(|&c| c > 0usize)
            .ok_or(VpiError::App("Invalid capacity given to array builder"))?;

        let item_type = self
            .item_type
            .ok_or(VpiError::App("Invalid item type given to array builder"))?;

        let item_size = unsafe { sys::vpiArrayTypeGetSize(item_type) } as usize;

        let stride_bytes = self
            .stride_bytes
            .or(Some(item_size))
            .filter(|&s| s > 0usize)
            .ok_or(VpiError::App("Invalid stride given to array builder"))?;

        // populate data struct
        let mut stable_size = Box::new(size as i32);
        let mut data = unsafe { std::mem::zeroed::<sys::VPIArrayData>() };

        data.bufferType = self
            .buffer_type
            .unwrap_or(sys::VPIArrayBufferType_VPI_ARRAY_BUFFER_HOST_AOS);

        data.buffer.aos.type_ = item_type;
        data.buffer.aos.data = data_ptr;
        data.buffer.aos.sizePointer = stable_size.as_mut();
        data.buffer.aos.capacity = capacity as i32;
        data.buffer.aos.strideBytes = stride_bytes as i32;

        Ok(ArrayDataWrapper {
            data,
            _size: stable_size,
        })
    }
}

pub struct BorrowedArray<'data> {
    inner: VpiArray,
    _data: ArrayDataWrapper,
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

/// Host data.
pub struct HostData<'arr> {
    inner: &'arr mut sys::VPIArrayBufferAOS,
}

impl<'arr> HostData<'arr> {
    pub fn as_ptr(&self) -> *mut c_void {
        self.inner.data
    }

    pub fn size(&self) -> usize {
        unsafe { *self.inner.sizePointer as usize }
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity as usize
    }

    pub fn stride_bytes(&self) -> usize {
        self.inner.strideBytes as usize
    }

    pub fn item_type(&self) -> sys::VPIArrayType {
        self.inner.type_
    }

    pub fn item_size(&self) -> usize {
        unsafe { sys::vpiArrayTypeGetSize(self.inner.type_) as usize }
    }

    pub fn size_bytes(&self) -> usize {
        self.size() * self.item_size()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.inner.data.cast(), self.size_bytes()) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.inner.data.cast(), self.size_bytes()) }
    }
}

/// Cuda data.
pub struct CudaData<'arr> {
    inner: &'arr mut sys::VPIArrayBufferAOS,
}

impl<'arr> CudaData<'arr> {
    pub fn size(&self) -> usize {
        unsafe { *self.inner.sizePointer as usize }
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity as usize
    }

    pub fn stride_bytes(&self) -> usize {
        self.inner.strideBytes as usize
    }

    pub fn item_type(&self) -> sys::VPIArrayType {
        self.inner.type_
    }

    pub fn item_size(&self) -> usize {
        unsafe { sys::vpiArrayTypeGetSize(self.inner.type_) as usize }
    }

    pub fn size_bytes(&self) -> usize {
        self.size() * self.item_size()
    }

    pub fn as_device_ptr(&self) -> *mut c_void {
        self.inner.data
    }
}

pub enum ArrayData<'arr> {
    Host(HostData<'arr>),
    Cuda(CudaData<'arr>),
}

pub struct ArrayLock<'arr> {
    /**
     * Either a view over a host buffer or a CUDA buffer.
     */
    array: &'arr mut VpiArray,
    data: sys::VPIArrayData,
}

impl<'arr> ArrayLock<'arr> {
    pub fn data(&mut self) -> ArrayData<'_> {
        match self.data.bufferType {
            sys::VPIArrayBufferType_VPI_ARRAY_BUFFER_HOST_AOS => ArrayData::Host(HostData {
                inner: &mut self.data.buffer.aos,
            }),
            sys::VPIArrayBufferType_VPI_ARRAY_BUFFER_CUDA_AOS => ArrayData::Cuda(CudaData {
                inner: &mut self.data.buffer.aos,
            }),
            // Invalid, if array is created a lock should never be invalid
            _ => unimplemented!(),
        }
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

    pub fn wrap<'data>(
        backing: &'data mut impl ArrayDataBacking,
        flags: u64,
    ) -> VpiResult<BorrowedArray<'data>> {
        let mut array_ptr = ptr::null_mut();
        let data_wrapper = backing.array_data()?;

        unsafe {
            check(sys::vpiArrayCreateWrapper(
                &raw const data_wrapper.data,
                flags,
                &raw mut array_ptr,
            ))?
        };

        let array = Self {
            handle: ptr::NonNull::new(array_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        };

        Ok(BorrowedArray {
            inner: array,
            _data: data_wrapper,
            _marker: PhantomData,
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
