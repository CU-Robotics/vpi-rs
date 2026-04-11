use crate::image::VpiImage;
use crate::stream::VpiStream;
use crate::sys;
use crate::util::{VpiResult, check};

pub fn submit_rescale(
    stream: &VpiStream,
    backend: u64,
    input: &VpiImage,
    output: &mut VpiImage,
    interpolation: sys::VPIInterpolationType,
    border: sys::VPIBorderExtension,
    flags: u64,
) -> VpiResult<()> {
    unsafe {
        check(sys::vpiSubmitRescale(
            stream.handle.as_ptr(),
            backend,
            input.handle.as_ptr(),
            output.handle.as_ptr(),
            interpolation,
            border,
            flags,
        ))
    }
}
