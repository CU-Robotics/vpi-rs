use crate::image::VpiImage;
use crate::stream::{VpiPayload, VpiStream};
use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

impl Default for sys::VPITNRParams {
    fn default() -> Self {
        let mut params = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiInitTemporalNoiseReductionParams(&raw mut params))
                .expect(util::FFI_SUCCESS_CONTRACT)
        };

        params
    }
}

pub struct TemporalNoiseReduction {
    payload: VpiPayload,
}

impl TemporalNoiseReduction {
    pub fn new(
        backends: u64,
        width: usize,
        height: usize,
        input_format: sys::VPIImageFormat,
        version: sys::VPITNRVersion,
    ) -> VpiResult<Self> {
        let mut tnr_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiCreateTemporalNoiseReduction(
                backends,
                width as i32,
                height as i32,
                input_format,
                version,
                &raw mut tnr_ptr,
            ))?
        };

        Ok(Self {
            payload: unsafe { VpiPayload::from_raw(tnr_ptr)? },
        })
    }

    pub fn submit(
        &self,
        stream: &VpiStream,
        backend: u64,
        previous_image: Option<&VpiImage>,
        input_image: &VpiImage,
        output_image: &mut VpiImage,
        params: &sys::VPITNRParams,
    ) -> VpiResult<()> {
        unsafe {
            check(sys::vpiSubmitTemporalNoiseReduction(
                stream.handle.as_ptr(),
                backend,
                self.payload.handle.as_ptr(),
                previous_image.map_or(ptr::null_mut(), |prev| prev.handle.as_ptr()),
                input_image.handle.as_ptr(),
                output_image.handle.as_ptr(),
                params,
            ))
        }
    }
}
