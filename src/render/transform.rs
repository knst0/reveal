#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    Fit,
    FitBest,
    Original,
    Free,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewTransform {
    pub zoom: f32,
    pub offset: (f32, f32),
    pub fit: FitMode,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self { zoom: 1.0, offset: (0.0, 0.0), fit: FitMode::Fit }
    }
}

impl ViewTransform {
    pub fn fit_zoom(image: (f32, f32), viewport: (f32, f32), mode: FitMode) -> f32 {
        Self::fit_zoom_with(image, viewport, mode, 1.0)
    }

    pub fn fit_zoom_with(
        image: (f32, f32),
        viewport: (f32, f32),
        mode: FitMode,
        original_zoom: f32,
    ) -> f32 {
        if image.0 <= 0.0 || image.1 <= 0.0 {
            return 1.0;
        }
        let scale = (viewport.0 / image.0).min(viewport.1 / image.1);
        match mode {
            FitMode::Original => original_zoom,
            FitMode::Fit => scale,
            FitMode::FitBest => scale.min(1.0),
            FitMode::Free => scale,
        }
    }

    pub fn apply_fit(&mut self, image: (f32, f32), viewport: (f32, f32)) {
        self.apply_fit_with(image, viewport, 1.0);
    }

    pub fn apply_fit_with(&mut self, image: (f32, f32), viewport: (f32, f32), original_zoom: f32) {
        if self.fit == FitMode::Free {
            return;
        }
        self.zoom = Self::fit_zoom_with(image, viewport, self.fit, original_zoom);
        self.offset = (0.0, 0.0);
    }

    pub fn set_fit(&mut self, fit: FitMode, image: (f32, f32), viewport: (f32, f32)) {
        self.set_fit_with(fit, image, viewport, 1.0);
    }

    pub fn set_fit_with(
        &mut self,
        fit: FitMode,
        image: (f32, f32),
        viewport: (f32, f32),
        original_zoom: f32,
    ) {
        self.fit = fit;
        self.apply_fit_with(image, viewport, original_zoom);
    }

    pub fn displayed_size(&self, image: (f32, f32)) -> (f32, f32) {
        (image.0 * self.zoom, image.1 * self.zoom)
    }

    pub fn image_bounds(&self, image: (f32, f32), viewport: (f32, f32)) -> (f32, f32, f32, f32) {
        let (w, h) = self.displayed_size(image);
        let x = (viewport.0 - w) / 2.0 + self.offset.0;
        let y = (viewport.1 - h) / 2.0 + self.offset.1;
        (x, y, w, h)
    }

    pub fn image_contains(
        &self,
        image: (f32, f32),
        viewport: (f32, f32),
        point: (f32, f32),
    ) -> bool {
        if image.0 <= 0.0 || image.1 <= 0.0 {
            return false;
        }
        let (x, y, w, h) = self.image_bounds(image, viewport);
        point.0 >= x && point.0 <= x + w && point.1 >= y && point.1 <= y + h
    }

    pub fn visible_source_rect(
        &self,
        image: (f32, f32),
        viewport: (f32, f32),
    ) -> Option<(u32, u32, u32, u32)> {
        if image.0 <= 0.0 || image.1 <= 0.0 || self.zoom <= 0.0 {
            return None;
        }
        let (x, y, w, h) = self.image_bounds(image, viewport);
        let left = ((-x).max(0.0) / self.zoom).floor().min(image.0);
        let top = ((-y).max(0.0) / self.zoom).floor().min(image.1);
        let right = (((viewport.0 - x).min(w)) / self.zoom).ceil().clamp(0.0, image.0);
        let bottom = (((viewport.1 - y).min(h)) / self.zoom).ceil().clamp(0.0, image.1);
        if right <= left || bottom <= top {
            return None;
        }
        Some((left as u32, top as u32, (right - left) as u32, (bottom - top) as u32))
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.fit = FitMode::Free;
        self.offset.0 += delta.0;
        self.offset.1 += delta.1;
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32), viewport: (f32, f32)) {
        let new_zoom = (self.zoom * factor).clamp(0.01, 100.0);
        let actual = new_zoom / self.zoom;
        let center = (viewport.0 / 2.0, viewport.1 / 2.0);
        let from_center =
            (cursor.0 - center.0 - self.offset.0, cursor.1 - center.1 - self.offset.1);
        self.offset.0 -= from_center.0 * (actual - 1.0);
        self.offset.1 -= from_center.1 * (actual - 1.0);
        self.zoom = new_zoom;
        self.fit = FitMode::Free;
    }
}
