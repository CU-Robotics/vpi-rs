use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

pub struct VpiContext {
    pub(crate) handle: ptr::NonNull<sys::VPIContextImpl>,
}

impl VpiContext {
    /**
     * Skip:
     * - vpiContextParallelFor
     * - vpiContextGetParallelFor
     *  ^ CPU backend, which we don't need
     * - vpiContextPush
     * - vpiContextPop
     *  ^ Ownership semantics are complicated
     */
    fn new(flags: u64) -> VpiResult<Self> {
        let mut context_ptr = ptr::null_mut();

        unsafe { check(sys::vpiContextCreate(flags, &raw mut context_ptr))? };

        Ok(Self {
            handle: ptr::NonNull::new(context_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    fn wrap_cuda(cuda_context: sys::CUcontext, flags: u64) -> VpiResult<Self> {
        let mut context_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiContextCreateWrapperCUDA(
                flags,
                cuda_context,
                &raw mut context_ptr,
            ))?
        };

        Ok(Self {
            handle: ptr::NonNull::new(context_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    fn current() -> VpiResult<Self> {
        let mut context_ptr = ptr::null_mut();

        unsafe { check(sys::vpiContextGetCurrent(&raw mut context_ptr))? };

        Ok(Self {
            handle: ptr::NonNull::new(context_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    fn set_current(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiContextSetCurrent(self.handle.as_ptr()))? };
        Ok(())
    }

    fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe {
            check(sys::vpiContextGetFlags(
                self.handle.as_ptr(),
                &raw mut flags,
            ))?
        };

        Ok(flags)
    }
}

impl Drop for VpiContext {
    fn drop(&mut self) {
        unsafe {
            sys::vpiContextDestroy(self.handle.as_ptr());
        }
    }
}
