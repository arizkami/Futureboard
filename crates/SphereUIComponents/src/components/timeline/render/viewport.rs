//! Arrangement-area viewport in content coordinates (lane region, excluding headers).

/// Scrollable arrangement viewport for the lane/grid region.
///
/// Coordinates are in **lane content space**: origin at the top-left of the
/// scrollable grid area (not including the fixed track-header column).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineViewport {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    /// Horizontal zoom: pixels per beat (derived from `pixels_per_second` × seconds/beat).
    pub pixels_per_beat: f32,
    pub pixels_per_second: f32,
    pub seconds_per_beat: f32,
}

impl TimelineViewport {
    pub fn new(
        width: f32,
        height: f32,
        scale_factor: f32,
        scroll_x: f32,
        scroll_y: f32,
        pixels_per_beat: f32,
        pixels_per_second: f32,
        seconds_per_beat: f32,
    ) -> Self {
        Self {
            width: width.max(1.0),
            height: height.max(1.0),
            scale_factor: scale_factor.max(1.0),
            scroll_x: scroll_x.max(0.0),
            scroll_y: scroll_y.max(0.0),
            pixels_per_beat: pixels_per_beat.max(0.0001),
            pixels_per_second: pixels_per_second.max(0.0001),
            seconds_per_beat: seconds_per_beat.max(0.0001),
        }
    }

    pub fn beat_to_x(&self, beat: f32) -> f32 {
        (beat * self.pixels_per_beat - self.scroll_x).round()
    }

    pub fn x_to_beat(&self, x: f32) -> f32 {
        ((x + self.scroll_x) / self.pixels_per_beat).max(0.0)
    }

    pub fn visible_beat_range(&self) -> (f32, f32) {
        let start = self.x_to_beat(0.0);
        let end = self.x_to_beat(self.width);
        (start, end.max(start))
    }
}
