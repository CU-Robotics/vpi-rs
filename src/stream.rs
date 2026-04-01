use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

pub struct VpiStream {
    handle: ptr::NonNull<sys::VPIStreamImpl>,
}

impl VpiStream {
    fn new(flags: u64) -> VpiResult<Self> {
        let mut stream_ptr = ptr::null_mut();

        unsafe { check(sys::vpiStreamCreate(flags, &raw mut stream_ptr))? };

        Ok(Self {
            handle: ptr::NonNull::new(stream_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    fn from_cuda_stream(cuda_stream: sys::CUstream, flags: u64) -> VpiResult<Self> {
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

    fn flush(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiStreamFlush(self.handle.as_ptr()))? };
        Ok(())
    }

    fn synchronize(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiStreamSync(self.handle.as_ptr()))? };
        Ok(())
    }

    // TODO: wait on event
}

impl Drop for VpiStream {
    fn drop(&mut self) {
        // SAFETY: no return value
        unsafe { sys::vpiStreamDestroy(self.handle.as_ptr()) };
    }
}
