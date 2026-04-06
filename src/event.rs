use crate::stream::VpiStream;
use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

pub struct VpiEvent {
    pub(crate) handle: ptr::NonNull<sys::VPIEventImpl>,
}

impl VpiEvent {
    fn new(flags: u64) -> VpiResult<Self> {
        let mut event_ptr = ptr::null_mut();

        unsafe { check(sys::vpiEventCreate(flags, &raw mut event_ptr))? };

        Ok(Self {
            handle: ptr::NonNull::new(event_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    fn record(&self, stream: &VpiStream) -> VpiResult<()> {
        unsafe {
            check(sys::vpiEventRecord(
                self.handle.as_ptr(),
                stream.handle.as_ptr(),
            ))?
        };

        Ok(())
    }

    fn synchronize(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiEventSync(self.handle.as_ptr()))? };

        Ok(())
    }

    // TODO: query

    fn elapsed_millis(&self, start: &VpiEvent) -> VpiResult<f32> {
        let mut elapsed_millis = f32::default();

        // treat self as the end
        unsafe {
            check(sys::vpiEventElapsedTimeMillis(
                start.handle.as_ptr(),
                self.handle.as_ptr(),
                &raw mut elapsed_millis,
            ))?
        };

        Ok(elapsed_millis)
    }

    fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe { check(sys::vpiEventGetFlags(self.handle.as_ptr(), &raw mut flags))? };

        Ok(flags)
    }
}

impl Drop for VpiEvent {
    fn drop(&mut self) {
        unsafe { sys::vpiEventDestroy(self.handle.as_ptr()) };
    }
}
