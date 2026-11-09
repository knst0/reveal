use crate::render::{FitMode, RenderConfig, Resample, ViewTransform};

pub struct ViewState {
    pub transform: ViewTransform,
    pub viewport: (f32, f32),
    pub scale_factor: f32,
    pub resample: Resample,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            transform: ViewTransform::default(),
            viewport: (0.0, 0.0),
            scale_factor: 1.0,
            resample: Resample::Filtered,
        }
    }
}

impl ViewState {
    pub fn dpr(&self) -> f32 {
        self.scale_factor.max(1.0)
    }

    pub fn physical_viewport(&self) -> (f32, f32) {
        let dpr = self.dpr();
        (self.viewport.0 * dpr, self.viewport.1 * dpr)
    }

    /// The single value shared by decode requests and display preparation.
    pub fn render_config(&self) -> RenderConfig {
        let physical = self.physical_viewport();
        RenderConfig {
            physical: (physical.0.max(1.0).round() as u32, physical.1.max(1.0).round() as u32),
            dpr: self.dpr(),
            resample: self.resample,
        }
    }

    pub fn antialias(&self) -> bool {
        self.resample == Resample::Filtered
    }

    pub fn apply_fit(&mut self, intrinsic: (f32, f32), original_zoom: f32) {
        self.transform.apply_fit_with(intrinsic, self.viewport, original_zoom);
    }

    pub fn set_fit(&mut self, fit: FitMode, intrinsic: (f32, f32), original_zoom: f32) {
        self.transform.set_fit_with(fit, intrinsic, self.viewport, original_zoom);
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.transform.pan(delta);
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32)) {
        self.transform.zoom_at(factor, cursor, self.viewport);
    }

    pub fn clamp_view(&mut self, image: (f32, f32)) {
        self.transform.clamp_offset(image, self.viewport);
    }
}
