use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ffi::c_void;
use std::marker::{PhantomData, PhantomPinned};
use std::ptr;

pub struct BorrowedImage<'data> {
    inner: VpiImage,
    _marker: PhantomData<&'data mut ()>,
}

/// Host pitch.
pub struct HostPlane<'img> {
    inner: &'img mut sys::VPIImagePlanePitchLinear,
}

impl<'img> HostPlane<'img> {
    pub fn pixel_type(&self) -> sys::VPIPixelType {
        self.inner.pixelType
    }

    pub fn width(&self) -> usize {
        self.inner.width as usize
    }

    pub fn height(&self) -> usize {
        self.inner.height as usize
    }

    pub fn pitch_bytes(&self) -> usize {
        self.inner.pitchBytes as usize
    }

    pub fn bits_per_pixel(&self) -> usize {
        unsafe { sys::vpiPixelTypeGetBitsPerPixel(self.inner.pixelType) as usize }
    }

    pub fn size_bytes(&self) -> usize {
        self.height().saturating_sub(1) * self.pitch_bytes()
            + self.width() * self.bits_per_pixel() / 8usize
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.inner.data.cast(), self.size_bytes()) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.inner.data.cast(), self.size_bytes()) }
    }
}

pub struct HostPitch<'img> {
    inner: &'img mut sys::VPIImageBufferPitchLinear,
}

impl<'img> HostPitch<'img> {
    pub fn planes_mut(&mut self) -> impl Iterator<Item = HostPlane<'_>> {
        self.inner.planes[..self.inner.numPlanes as usize]
            .iter_mut()
            .map(|p| HostPlane { inner: p })
    }
}

/// Cuda pitch.
pub struct CudaPlane<'img> {
    inner: &'img sys::VPIImagePlanePitchLinear,
}

impl<'img> CudaPlane<'img> {
    pub fn pixel_type(&self) -> sys::VPIPixelType {
        self.inner.pixelType
    }

    pub fn width(&self) -> usize {
        self.inner.width as usize
    }

    pub fn height(&self) -> usize {
        self.inner.height as usize
    }

    pub fn pitch_bytes(&self) -> usize {
        self.inner.pitchBytes as usize
    }

    pub fn bits_per_pixel(&self) -> usize {
        unsafe { sys::vpiPixelTypeGetBitsPerPixel(self.inner.pixelType) as usize }
    }

    pub fn size_bytes(&self) -> usize {
        self.height().saturating_sub(1) * self.pitch_bytes()
            + self.width() * self.bits_per_pixel() / 8usize
    }

    pub fn as_device_ptr(&self) -> *mut c_void {
        self.inner.data
    }
}

pub struct CudaPitch<'img> {
    inner: &'img sys::VPIImageBufferPitchLinear,
}

impl<'img> CudaPitch<'img> {
    pub fn planes(&self) -> impl Iterator<Item = CudaPlane<'_>> {
        self.inner.planes[..self.inner.numPlanes as usize]
            .iter()
            .map(|p| CudaPlane { inner: p })
    }
}

/// Cuda array.
pub struct CudaArray<'img> {
    inner: sys::cudaArray_t,
    _marker: PhantomData<&'img mut ()>,
}

impl<'img> CudaArray<'img> {
    pub fn as_device_ptr(&self) -> sys::cudaArray_t {
        self.inner
    }
}

pub enum ImageData<'img> {
    HostPitchLinear(HostPitch<'img>),
    CudaPitchLinear(CudaPitch<'img>),
    CudaArray(CudaArray<'img>),
}

pub struct ImageLock<'img> {
    image: &'img mut VpiImage,
    data: sys::VPIImageData,
}

impl<'img> ImageLock<'img> {
    pub fn data(&mut self) -> ImageData<'_> {
        match self.data.bufferType {
            sys::VPIImageBufferType_VPI_IMAGE_BUFFER_HOST_PITCH_LINEAR => {
                ImageData::HostPitchLinear(HostPitch {
                    inner: unsafe { &mut self.data.buffer.pitch },
                })
            }
            sys::VPIImageBufferType_VPI_IMAGE_BUFFER_CUDA_PITCH_LINEAR => {
                ImageData::CudaPitchLinear(CudaPitch {
                    inner: unsafe { &self.data.buffer.pitch },
                })
            }
            sys::VPIImageBufferType_VPI_IMAGE_BUFFER_CUDA_ARRAY => {
                ImageData::CudaArray(CudaArray {
                    inner: unsafe { self.data.buffer.cudaarray },
                    _marker: PhantomData,
                })
            }
            _ => unimplemented!(),
        }
    }
}

impl Drop for ImageLock<'_> {
    fn drop(&mut self) {
        let _ = self.image.unlock();
    }
}

pub struct VpiImage {
    pub(crate) handle: ptr::NonNull<sys::VPIImageImpl>,
}

impl VpiImage {
    pub fn new(
        width: usize,
        height: usize,
        format: sys::VPIImageFormat,
        flags: u64,
    ) -> VpiResult<Self> {
        let mut image_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiImageCreate(
                width as i32,
                height as i32,
                format,
                flags,
                &raw mut image_ptr,
            ))?
        };

        Ok(Self {
            handle: ptr::NonNull::new(image_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        })
    }

    pub fn get_size(&self) -> VpiResult<(usize, usize)> {
        let (mut width, mut height) = <(i32, i32)>::default();

        unsafe {
            check(sys::vpiImageGetSize(
                self.handle.as_ptr(),
                &raw mut width,
                &raw mut height,
            ))?
        };

        Ok((width as usize, height as usize))
    }

    pub fn get_format(&self) -> VpiResult<sys::VPIImageFormat> {
        let mut format = sys::VPIImageFormat::default();

        unsafe {
            check(sys::vpiImageGetFormat(
                self.handle.as_ptr(),
                &raw mut format,
            ))?
        };

        Ok(format)
    }

    pub fn get_flags(&self) -> VpiResult<u64> {
        let mut flags = u64::default();

        unsafe { check(sys::vpiImageGetFlags(self.handle.as_ptr(), &raw mut flags))? };

        Ok(flags)
    }

    fn unlock(&self) -> VpiResult<()> {
        unsafe { check(sys::vpiImageUnlock(self.handle.as_ptr()))? }
        Ok(())
    }

    pub fn lock<'img>(
        &mut self,
        mode: sys::VPILockMode,
        buffer_type: sys::VPIImageBufferType,
    ) -> VpiResult<ImageLock<'_>> {
        let mut data = unsafe { std::mem::zeroed() };

        unsafe {
            check(sys::vpiImageLockData(
                self.handle.as_ptr(),
                mode,
                buffer_type,
                &raw mut data,
            ))?
        };

        Ok(ImageLock { image: self, data })
    }
}

impl Drop for VpiImage {
    fn drop(&mut self) {
        unsafe {
            sys::vpiImageDestroy(self.handle.as_ptr());
        }
    }
}
