use std::sync::Arc;

use gpui::{
    App, Bounds, Corners, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, RenderImage, Size, Style, Window, px,
};

use super::ViewTransform;

pub struct ImageElement {
    image: Option<Arc<RenderImage>>,
    intrinsic: (f32, f32),
    transform: ViewTransform,
    frame_index: usize,
    crop: Option<(u32, u32, u32, u32)>,
}

pub struct PaintPlan {
    bounds: Bounds<Pixels>,
}

impl ImageElement {
    pub fn new(
        image: Option<Arc<RenderImage>>,
        intrinsic: (f32, f32),
        transform: ViewTransform,
        frame_index: usize,
        crop: Option<(u32, u32, u32, u32)>,
    ) -> Self {
        Self { image, intrinsic, transform, frame_index, crop }
    }
}

impl IntoElement for ImageElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ImageElement {
    type RequestLayoutState = ();
    type PrepaintState = PaintPlan;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style { size: Size::full(), ..Default::default() };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let viewport = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let transform = self.transform;

        let (x, y, w, h) = transform.image_bounds(self.intrinsic, viewport);
        let (x, y, w, h) = match self.crop {
            Some((cx, cy, cw, ch)) => (
                x + cx as f32 * transform.zoom,
                y + cy as f32 * transform.zoom,
                cw as f32 * transform.zoom,
                ch as f32 * transform.zoom,
            ),
            None => (x, y, w, h),
        };
        PaintPlan {
            bounds: Bounds {
                origin: gpui::point(bounds.origin.x + px(x), bounds.origin.y + px(y)),
                size: gpui::size(px(w), px(h)),
            },
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let Some(image) = self.image.clone() else {
            return;
        };
        if let Err(e) = window.paint_image(
            prepaint.bounds,
            prepaint.bounds,
            Corners::default(),
            image,
            self.frame_index,
            false,
        ) {
            log::error!("paint_image failed: {e}");
        }
    }
}
