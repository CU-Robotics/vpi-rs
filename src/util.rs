use crate::sys::{self, VPIStatus};
use thiserror::Error;

pub const FFI_SUCCESS_CONTRACT: &str = "Status was VPI_SUCCESS";

#[derive(Debug, Error)]
#[error("VpiError: {name} ({status}); {message}")]
pub struct VpiSysError {
    status: VPIStatus,
    name: String,
    message: String,
}

#[derive(Debug, Error)]
pub enum VpiError {
    #[error("System error: {0}")]
    Sys(#[from] VpiSysError),
    #[error("Application error: {0}")]
    App(&'static str),
}

pub type VpiResult<T> = Result<T, VpiError>;

impl From<VPIStatus> for VpiSysError {
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
            sys::vpiGetLastStatusMessage(message_buffer.as_mut_ptr().cast(), message_buffer.len() as i32)
        };

        // SAFETY: message buffer can safely be converted to CStr
        let message = unsafe {
            std::ffi::CStr::from_ptr(message_buffer.as_ptr().cast())
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

pub fn check(status: sys::VPIStatus) -> VpiResult<()> {
    if status == sys::VPIStatus_VPI_SUCCESS {
        Ok(())
    } else {
        Err(Into::<VpiSysError>::into(status).into())
    }
}
