use crate::image::VpiImage;
use crate::stream::VpiStream;
use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ptr;

impl Default for sys::VPIConvertImageFormatParams {
    fn default() -> Self {
        let mut params = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiInitConvertImageFormatParams(&raw mut params))
                .expect(util::FFI_SUCCESS_CONTRACT)
        };

        params
    }
}

pub fn submit_format_conversion(
    stream: &VpiStream,
    backend: u64,
    input: &VpiImage,
    output: &mut VpiImage,
) -> VpiResult<()> {
    unsafe {
        check(sys::vpiSubmitConvertImageFormat(
            stream.handle.as_ptr(),
            backend,
            input.handle.as_ptr(),
            output.handle.as_ptr(),
            ptr::null(),
        ))
    }
}

pub fn submit_custom_format_conversion(
    stream: &VpiStream,
    backend: u64,
    input: &VpiImage,
    output: &mut VpiImage,
    params: &sys::VPIConvertImageFormatParams,
) -> VpiResult<()> {
    unsafe {
        check(sys::vpiSubmitConvertImageFormat(
            stream.handle.as_ptr(),
            backend,
            input.handle.as_ptr(),
            output.handle.as_ptr(),
            params,
        ))
    }
}
