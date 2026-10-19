//! The camera over the card map: a world point at the map region's center, and a zoom.
//!
//! World coordinates are points at zoom 1: one layout cell is [`CELL`], so a card is 168 x 108 points.
//! Screen coordinates are egui points. The camera never changes the layout: a card's world rect is fixed.

use eframe::egui::{Pos2, Rect, Vec2, vec2};

/// One layout cell in world points.
pub const CELL: Vec2 = vec2(8.0, 18.0);
/// The fit view keeps this margin around the map, in screen points.
pub const FIT_MARGIN: f32 = 24.0;
/// The largest zoom: a card is 1,008 points wide.
pub const MAX_ZOOM: f32 = 6.0;
/// `+` and `-` zoom by this factor around the map region's center.
pub const KEY_ZOOM: f32 = 1.5;
/// A mouse wheel notch zooms by this factor around the pointer.
pub const WHEEL_ZOOM: f32 = 1.15;
/// A trackpad scroll with Cmd zooms by `exp(points / SCROLL_ZOOM_POINTS)`.
pub const SCROLL_ZOOM_POINTS: f32 = 200.0;
/// `w` `a` `s` `d` pan this share of the view, and this share with Shift.
pub const PAN_STEP: f32 = 0.10;
pub const PAN_FAR: f32 = 0.40;
/// Key zooms and fits ease out over this time, in seconds.
pub const KEY_EASE: f64 = 0.160;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The world point at the map region's center.
    pub center: Pos2,
    /// Screen points per world point.
    pub zoom: f32,
}

impl Camera {
    pub fn to_screen(&self, region: Rect, p: Pos2) -> Pos2 {
        region.center() + (p - self.center) * self.zoom
    }

    pub fn to_world(&self, region: Rect, s: Pos2) -> Pos2 {
        self.center + (s - region.center()) / self.zoom
    }

    pub fn rect_to_screen(&self, region: Rect, r: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(region, r.min), self.to_screen(region, r.max))
    }

    /// The world rect that the map region shows.
    pub fn view(&self, region: Rect) -> Rect {
        Rect::from_min_max(
            self.to_world(region, region.min),
            self.to_world(region, region.max),
        )
    }
}

/// The map's world size, and the camera limits that follow from it and a map region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub map: Vec2,
}

impl Bounds {
    /// The whole map in the region, with [`FIT_MARGIN`].
    pub fn fit(&self, region: Rect) -> Camera {
        self.fit_rect(region, Rect::from_min_size(Pos2::ZERO, self.map))
    }

    /// A world rect in the region, with [`FIT_MARGIN`], within the zoom range.
    pub fn fit_rect(&self, region: Rect, r: Rect) -> Camera {
        let avail = (region.size() - Vec2::splat(2.0 * FIT_MARGIN)).max(Vec2::splat(1.0));
        let size = r.size().max(Vec2::splat(1.0));
        let zoom = (avail.x / size.x).min(avail.y / size.y);
        self.clamp(
            region,
            Camera {
                center: r.center(),
                zoom,
            },
        )
    }

    /// The zoom range: from half the fit zoom to [`MAX_ZOOM`].
    pub fn zoom_range(&self, region: Rect) -> (f32, f32) {
        let avail = (region.size() - Vec2::splat(2.0 * FIT_MARGIN)).max(Vec2::splat(1.0));
        let fit = (avail.x / self.map.x.max(1.0)).min(avail.y / self.map.y.max(1.0));
        (fit / 2.0, MAX_ZOOM.max(fit / 2.0))
    }

    /// Keeps the zoom in its range and the region's center inside the map.
    pub fn clamp(&self, region: Rect, cam: Camera) -> Camera {
        let (lo, hi) = self.zoom_range(region);
        Camera {
            center: Pos2::new(
                cam.center.x.clamp(0.0, self.map.x),
                cam.center.y.clamp(0.0, self.map.y),
            ),
            zoom: cam.zoom.clamp(lo, hi),
        }
    }

    /// Zooms by `factor`, so the world point under the screen point `at` stays under it.
    pub fn zoom_around(&self, region: Rect, cam: Camera, at: Pos2, factor: f32) -> Camera {
        let (lo, hi) = self.zoom_range(region);
        let world = cam.to_world(region, at);
        let zoom = (cam.zoom * factor).clamp(lo, hi);
        self.clamp(
            region,
            Camera {
                center: world - (at - region.center()) / zoom,
                zoom,
            },
        )
    }

    /// Moves the content by `delta` screen points.
    pub fn pan(&self, region: Rect, cam: Camera, delta: Vec2) -> Camera {
        self.clamp(
            region,
            Camera {
                center: cam.center - delta / cam.zoom,
                ..cam
            },
        )
    }
}

fn ease_out(t: f64) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - (1.0 - t).powi(3)) as f32
}

/// The camera between two cameras: the center moves linearly, and the zoom geometrically.
pub fn between(a: Camera, b: Camera, e: f32) -> Camera {
    Camera {
        center: a.center + (b.center - a.center) * e,
        zoom: a.zoom * (b.zoom / a.zoom).powf(e),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ease {
    from: Camera,
    to: Camera,
    start: f64,
    dur: f64,
}

/// The camera and its ease-out toward a target. Times are seconds, as egui's `InputState::time`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub cam: Camera,
    ease: Option<Ease>,
}

impl Motion {
    pub fn new(cam: Camera) -> Motion {
        Motion { cam, ease: None }
    }

    /// Where the camera goes: the end of the ease, else the camera.
    pub fn target(&self) -> Camera {
        self.ease.map_or(self.cam, |e| e.to)
    }

    pub fn easing(&self) -> bool {
        self.ease.is_some()
    }

    /// Sets the camera now, and stops any ease.
    pub fn jump(&mut self, cam: Camera) {
        self.cam = cam;
        self.ease = None;
    }

    /// Eases from the camera to `to` over `dur` seconds.
    pub fn ease_to(&mut self, to: Camera, now: f64, dur: f64) {
        self.ease = Some(Ease {
            from: self.cam,
            to,
            start: now,
            dur,
        });
    }

    /// Moves the camera and the ease's end by the same world offset, then clamps both.
    pub fn shift(&mut self, bounds: &Bounds, region: Rect, by: Vec2) {
        let move_cam = |c: Camera| {
            bounds.clamp(
                region,
                Camera {
                    center: c.center + by,
                    ..c
                },
            )
        };
        self.cam = move_cam(self.cam);
        if let Some(e) = &mut self.ease {
            e.from = move_cam(e.from);
            e.to = move_cam(e.to);
        }
    }

    /// Moves the camera and the ease's ends so that each world point keeps its screen point after the map region's center
    /// moved by `d` screen points, then clamps them to the new region (unit `042`).
    pub fn keep_on_screen(&mut self, bounds: &Bounds, region: Rect, d: Vec2) {
        let move_cam = |c: Camera| {
            bounds.clamp(
                region,
                Camera {
                    center: c.center + d / c.zoom,
                    ..c
                },
            )
        };
        self.cam = move_cam(self.cam);
        if let Some(e) = &mut self.ease {
            e.from = move_cam(e.from);
            e.to = move_cam(e.to);
        }
    }

    /// Advances the ease to `now`. Returns true while it runs.
    pub fn tick(&mut self, now: f64) -> bool {
        let Some(e) = self.ease else {
            return false;
        };
        let t = if e.dur > 0.0 {
            (now - e.start) / e.dur
        } else {
            1.0
        };
        if t >= 1.0 {
            self.cam = e.to;
            self.ease = None;
            return false;
        }
        self.cam = between(e.from, e.to, ease_out(t));
        true
    }
}
