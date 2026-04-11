use crate::sys;
use crate::util::{VpiResult, check};
use std::ptr;

struct GridRegion {
    size: i16,
    /// Must be PO2, >=1.
    interval: i16,
}

enum GenerationFn {
    Identity,
    Fisheye {
        k: sys::VPICameraIntrinsic,
        x: sys::VPICameraExtrinsic,
        k_new: sys::VPICameraIntrinsic,
        model: sys::VPIFisheyeLensDistortionModel,
    },
    Polynomial {
        k: sys::VPICameraIntrinsic,
        x: sys::VPICameraExtrinsic,
        k_new: sys::VPICameraIntrinsic,
        model: sys::VPIPolynomialLensDistortionModel,
    },
}

#[derive(Default)]
pub struct WarpMapBuilder {
    vertical_regions: Vec<GridRegion>,
    horizontal_regions: Vec<GridRegion>,
    generation_fn: Option<GenerationFn>,
}

impl WarpMapBuilder {
    pub fn vert_region(mut self, height: i16, interval: i16) -> Self {
        self.vertical_regions.push(GridRegion {
            size: height,
            interval,
        });
        self
    }

    pub fn horiz_region(mut self, width: i16, interval: i16) -> Self {
        self.horizontal_regions.push(GridRegion {
            size: width,
            interval,
        });
        self
    }

    pub fn identity(mut self) -> Self {
        self.generation_fn = Some(GenerationFn::Identity);
        self
    }

    pub fn fisheye(
        mut self,
        k: sys::VPICameraIntrinsic,
        x: sys::VPICameraExtrinsic,
        k_new: sys::VPICameraIntrinsic,
        model: sys::VPIFisheyeLensDistortionModel,
    ) -> Self {
        self.generation_fn = Some(GenerationFn::Fisheye { k, x, k_new, model });
        self
    }

    pub fn polynomial(
        mut self,
        k: sys::VPICameraIntrinsic,
        x: sys::VPICameraExtrinsic,
        k_new: sys::VPICameraIntrinsic,
        model: sys::VPIPolynomialLensDistortionModel,
    ) -> Self {
        self.generation_fn = Some(GenerationFn::Polynomial { k, x, k_new, model });
        self
    }

    pub fn build(self) -> VpiResult<VpiWarpMap> {
        let mut map = unsafe { std::mem::zeroed::<sys::VPIWarpMap>() };

        let num_horiz_regions = self.horizontal_regions.len();
        let num_vert_regions = self.vertical_regions.len();

        map.grid.numHorizRegions = num_horiz_regions as i8;
        map.grid.numVertRegions = num_vert_regions as i8;

        for (i, GridRegion { size, interval }) in self
            .horizontal_regions
            .into_iter()
            .take(sys::VPI_WARPGRID_MAX_HORIZ_REGIONS_COUNT as usize)
            .enumerate()
        {
            map.grid.regionWidth[i] = size;
            map.grid.horizInterval[i] = interval;
        }

        for (i, GridRegion { size, interval }) in self
            .vertical_regions
            .into_iter()
            .take(sys::VPI_WARPGRID_MAX_VERT_REGIONS_COUNT as usize)
            .enumerate()
        {
            map.grid.regionHeight[i] = size;
            map.grid.vertInterval[i] = interval;
        }

        map.keypoints = ptr::null_mut();

        // Will throw an error if any parameters are invalid
        unsafe { check(sys::vpiWarpMapAllocData(&raw mut map))? };

        // Generate keypoints
        unsafe {
            match self.generation_fn.unwrap_or(GenerationFn::Identity) {
                GenerationFn::Identity => check(sys::vpiWarpMapGenerateIdentity(&raw mut map))?,
                GenerationFn::Fisheye { k, x, k_new, model } => {
                    check(sys::vpiWarpMapGenerateFromFisheyeLensDistortionModel(
                        k.as_ptr(),
                        x.as_ptr(),
                        k_new.as_ptr(),
                        &raw const model,
                        &raw mut map,
                    ))?;
                }
                GenerationFn::Polynomial { k, x, k_new, model } => {
                    check(sys::vpiWarpMapGenerateFromPolynomialLensDistortionModel(
                        k.as_ptr(),
                        x.as_ptr(),
                        k_new.as_ptr(),
                        &raw const model,
                        &raw mut map,
                    ))?;
                }
            }
        }

        Ok(VpiWarpMap { handle: map })
    }
}

pub struct VpiWarpMap {
    pub(crate) handle: sys::VPIWarpMap,
}

impl VpiWarpMap {
    pub fn num_horiz_points(&self) -> usize {
        self.handle.numHorizPoints as usize
    }

    pub fn num_vert_points(&self) -> usize {
        self.handle.numVertPoints as usize
    }

    pub fn pitch_bytes(&self) -> usize {
        self.handle.pitchBytes as usize
    }

    fn rows_mut(&mut self) -> impl Iterator<Item = &mut [sys::VPIKeypointF32]> {
        let base_ptr = self.handle.keypoints.cast::<u8>();
        let pitch = self.pitch_bytes();
        let width = self.num_horiz_points();
        let height = self.num_vert_points();

        (0..height).map(move |i| {
            let row_ptr = unsafe { base_ptr.add(i * pitch).cast::<sys::VPIKeypointF32>() };

            unsafe { std::slice::from_raw_parts_mut(row_ptr, width) }
        })
    }

    pub fn keypoints_mut(&mut self) -> impl Iterator<Item = &mut sys::VPIKeypointF32> {
        self.rows_mut().flat_map(|row| row.iter_mut())
    }
}

impl Drop for VpiWarpMap {
    fn drop(&mut self) {
        unsafe {
            sys::vpiWarpMapFreeData(&raw mut self.handle);
        }
    }
}
