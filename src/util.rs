use crate::sys::{self, VPIStatus};

pub const FFI_SUCCESS_CONTRACT: &str = "Status was VPI_SUCCESS";

#[derive(Debug)]
pub struct VpiError {
    status: VPIStatus,
    name: String,
    message: String,
}

pub type VpiResult<T> = Result<T, VpiError>;

impl From<VPIStatus> for VpiError {
    fn from(status: VPIStatus) -> Self {
        // SAFETY: shouldn't fail
        let name_ptr = unsafe { sys::vpiStatusGetName(status) };

        let name = if name_ptr.is_null() {
            "Unknown".to_owned()
        } else {
            // SAFETY: name can safely be converted to CStr
            unsafe {
                std::ffi::CStr::from_ptr(name_ptr)
                    .to_string_lossy()
                    .into_owned()
            }
        };

        let mut message_buffer = vec![0u8; sys::VPI_MAX_STATUS_MESSAGE_LENGTH as usize];

        // we already have the status, ignore return value
        // SAFETY: shouldn't fail
        let _ = unsafe {
            sys::vpiGetLastStatusMessage(
                message_buffer.as_mut_ptr().cast::<i8>(),
                message_buffer.len() as i32,
            )
        };

        // SAFETY: message buffer can safely be converted to CStr
        let message = unsafe {
            std::ffi::CStr::from_ptr(message_buffer.as_ptr().cast::<i8>())
                .to_string_lossy()
                .into_owned()
        };

        Self {
            status,
            name,
            message,
        }
    }
}

impl std::fmt::Display for VpiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VpiError: {} ({}); {}",
            self.name, self.status, self.message
        )
    }
}

pub fn check(status: sys::VPIStatus) -> VpiResult<()> {
    if status == sys::VPIStatus_VPI_SUCCESS {
        Ok(())
    } else {
        Err(status.into())
    }
}

pub enum AllocType {
    Host,
    Cuda,
}
