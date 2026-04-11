use crate::image::VpiImage;
use crate::stream::{VpiPayload, VpiStream};
use crate::sys;
use crate::util::{VpiResult, check};
use crate::warp_map::VpiWarpMap;
use std::ptr;

pub struct Remap {
    payload: VpiPayload,
}

impl Remap {
    pub fn new(backends: u64, map: &VpiWarpMap) -> VpiResult<Self> {
        let mut payload_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiCreateRemap(
                backends,
                &raw const map.handle,
                &raw mut payload_ptr,
            ))?
        };

        Ok(Self {
            payload: unsafe { VpiPayload::from_raw(payload_ptr)? },
        })
    }

    pub fn submit(
        &self,
        stream: &VpiStream,
        backend: u64,
        input: &VpiImage,
        output: &mut VpiImage,
        interpolation: sys::VPIInterpolationType,
        border: sys::VPIBorderExtension,
        flags: u64,
    ) -> VpiResult<()> {
        unsafe {
            check(sys::vpiSubmitRemap(
                stream.handle.as_ptr(),
                backend,
                self.payload.handle.as_ptr(),
                input.handle.as_ptr(),
                output.handle.as_ptr(),
                interpolation,
                border,
                flags,
            ))
        }
    }
}
