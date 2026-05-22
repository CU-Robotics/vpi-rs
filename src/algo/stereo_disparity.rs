use crate::image::VpiImage;
use crate::stream::{VpiPayload, VpiStream};
use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

impl Default for sys::VPIStereoDisparityEstimatorCreationParams {
    fn default() -> Self {
        let mut params = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiInitStereoDisparityEstimatorCreationParams(
                &raw mut params,
            ))
            .expect(util::FFI_SUCCESS_CONTRACT)
        };

        params
    }
}

impl Default for sys::VPIStereoDisparityEstimatorParams {
    fn default() -> Self {
        let mut params = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiInitStereoDisparityEstimatorParams(&raw mut params))
                .expect(util::FFI_SUCCESS_CONTRACT)
        };

        params
    }
}

pub struct StereoDisparityEstimator {
    payload: VpiPayload,
}

impl StereoDisparityEstimator {
    pub fn new(
        backends: u64,
        width: usize,
        height: usize,
        input_format: sys::VPIImageFormat,
        params: sys::VPIStereoDisparityEstimatorCreationParams,
    ) -> VpiResult<Self> {
        let mut estimator_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiCreateStereoDisparityEstimator(
                backends,
                width as i32,
                height as i32,
                input_format,
                &raw const params,
                &raw mut estimator_ptr,
            ))?
        };

        Ok(Self {
            payload: unsafe { VpiPayload::from_raw(estimator_ptr)? },
        })
    }

    pub fn submit(
        &self,
        stream: &VpiStream,
        backend: u64,
        left: &VpiImage,
        right: &VpiImage,
        disparity: &mut VpiImage,
        params: &sys::VPIStereoDisparityEstimatorParams,
    ) -> VpiResult<()> {
        unsafe {
            check(sys::vpiSubmitStereoDisparityEstimator(
                stream.handle.as_ptr(),
                backend,
                self.payload.handle.as_ptr(),
                left.handle.as_ptr(),
                right.handle.as_ptr(),
                disparity.handle.as_ptr(),
                ptr::null_mut(),
                params,
            ))
        }
    }
}
