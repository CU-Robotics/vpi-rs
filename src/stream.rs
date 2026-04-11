use crate::event::VpiEvent;
use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

pub struct VpiPayload {
    pub(crate) handle: ptr::NonNull<sys::VPIPayloadImpl>,
}

impl VpiPayload {
    pub(crate) unsafe fn from_raw(payload_ptr: *mut sys::VPIPayloadImpl) -> Self {
        Self {
            handle: ptr::NonNull::new(payload_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        }
    }

    pub fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe {
            check(sys::vpiPayloadGetFlags(
                self.handle.as_ptr(),
                &raw mut flags,
            ))?
        };

        Ok(flags)
    }
}

impl Drop for VpiPayload {
    fn drop(&mut self) {
        unsafe { sys::vpiPayloadDestroy(self.handle.as_ptr()) };
    }
}

pub struct VpiStream {
    pub(crate) handle: ptr::NonNull<sys::VPIStreamImpl>,
}

impl VpiStream {
    /**
     * Skip:
     * - vpiStreamGetThreadHandle
     *  ^ Don't need right now
     */
    pub fn new(flags: u64) -> VpiResult<Self> {
        let mut stream_ptr = ptr::null_mut();

        unsafe { check(sys::vpiStreamCreate(flags, &raw mut stream_ptr))? };

        Ok(Self {
            handle: ptr::NonNull::new(stream_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    pub fn wrap_cuda(cuda_stream: sys::CUstream, flags: u64) -> VpiResult<Self> {
        let mut stream_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiStreamCreateWrapperCUDA(
                cuda_stream,
                flags,
                &raw mut stream_ptr,
            ))?
        };

        Ok(Self {
            handle: ptr::NonNull::new(stream_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    pub fn flush(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiStreamFlush(self.handle.as_ptr()))? };
        Ok(())
    }

    pub fn synchronize(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiStreamSync(self.handle.as_ptr()))? };
        Ok(())
    }

    pub fn wait_event(&self, event: &VpiEvent) -> VpiResult<()> {
        unsafe {
            check(sys::vpiStreamWaitEvent(
                self.handle.as_ptr(),
                event.handle.as_ptr(),
            ))?
        };
        Ok(())
    }

    pub fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe { check(sys::vpiStreamGetFlags(self.handle.as_ptr(), &raw mut flags))? };

        Ok(flags)
    }
}

impl Drop for VpiStream {
    fn drop(&mut self) {
        // SAFETY: no return value
        unsafe { sys::vpiStreamDestroy(self.handle.as_ptr()) };
    }
}
