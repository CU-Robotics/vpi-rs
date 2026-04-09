use crate::sys;
use crate::util::{self, VpiResult, check};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr;

pub trait ImageDataBacking {
    fn image_data(&self) -> sys::VPIImageData;
}

#[derive(Default)]
pub struct ImageDataBuilder {
    buffer_type: Option<sys::VPIImageBufferType>,
    cuda_array: Option<sys::cudaArray_t>,
    pitch: Vec<sys::VPIImagePlanePitchLinear>,
}

impl ImageDataBuilder {
    pub fn cuda(mut self) -> Self {
        self.buffer_type = Some(sys::VPIImageBufferType_VPI_IMAGE_BUFFER_CUDA_PITCH_LINEAR);
        self
    }

    pub fn array(mut self, array: sys::cudaArray_t) -> Self {
        self.cuda_array = Some(array);
        self
    }

    pub unsafe fn plane(
        mut self,
        data: *mut c_void,
        width: usize,
        height: usize,
        pitch_bytes: usize,
        pixel_type: sys::VPIPixelType,
    ) -> Self {
        self.pitch.push(sys::VPIImagePlanePitchLinear {
            pixelType: pixel_type,
            width: width as i32,
            height: height as i32,
            pitchBytes: pitch_bytes as i32,
            data,
        });
        self
    }
}

impl ImageDataBacking for ImageDataBuilder {
    fn image_data(&self) -> sys::VPIImageData {
        let mut data: sys::VPIImageData = unsafe { std::mem::zeroed() };

        // handle cudaArray_t
        if let Some(array) = self.cuda_array {
            data.bufferType = sys::VPIImageBufferType_VPI_IMAGE_BUFFER_CUDA_ARRAY;
            data.buffer.cudaarray = array;

            return data;
        }

        // handle pitched data
        data.bufferType = self
            .buffer_type
            .unwrap_or(sys::VPIImageBufferType_VPI_IMAGE_BUFFER_HOST_PITCH_LINEAR);

        data.buffer.pitch.numPlanes = self.pitch.len() as i32;

        let max_planes = unsafe { data.buffer.pitch.planes.len() };
        for (i, p) in self.pitch.iter().take(max_planes).enumerate() {
            unsafe {
                data.buffer.pitch.planes[i] = *p;
            }
        }

        data
    }
}

pub struct BorrowedImage<'data> {
    inner: VpiImage,
    _marker: PhantomData<&'data mut ()>,
}

impl Deref for BorrowedImage<'_> {
    type Target = VpiImage;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BorrowedImage<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
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
            // Don't really care about NvBuffer or EGL right now.
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
    /**
     * Skip:
     * - vpiImageSetView
     * - vpiImageSetWrapper
     *  ^ Ownership semantics are complicated
     * - vpiImageLock
     *  ^ Lock without data pretty much useless
     */
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

    pub fn wrap<'data>(
        backing: &'data mut impl ImageDataBacking,
        flags: u64,
    ) -> VpiResult<BorrowedImage<'data>> {
        let mut image_ptr = ptr::null_mut();

        let image_data = backing.image_data();

        unsafe {
            check(sys::vpiImageCreateWrapper(
                &raw const image_data,
                ptr::null(),
                flags,
                &raw mut image_ptr,
            ))?
        };

        let image = Self {
            handle: ptr::NonNull::new(image_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        };

        Ok(BorrowedImage {
            inner: image,
            _marker: PhantomData,
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

    pub fn lock(
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

    pub fn get_roi(&mut self, roi: sys::VPIRectangleI, flags: u64) -> VpiResult<BorrowedImage<'_>> {
        let mut roi_ptr = ptr::null_mut();

        unsafe {
            check(sys::vpiImageCreateView(
                self.handle.as_ptr(),
                &raw const roi,
                flags,
                &raw mut roi_ptr,
            ))?
        };

        let image = Self {
            handle: ptr::NonNull::new(roi_ptr).expect(util::FFI_SUCCESS_CONTRACT),
        };

        Ok(BorrowedImage {
            inner: image,
            _marker: PhantomData,
        })
    }
}

impl Drop for VpiImage {
    fn drop(&mut self) {
        unsafe {
            sys::vpiImageDestroy(self.handle.as_ptr());
        }
    }
}
