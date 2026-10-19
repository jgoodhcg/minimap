//! Capture mode for visual checks (`--capture DIR`): a fixed script of camera and toggle steps, saved as PNGs,
//! then two zoom sweeps whose frame CPU times and font atlas sizes go to `DIR/frames.txt`. Then the window closes.
//! A step waits for the details of its visible cards before its screenshot.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use eframe::egui::{self, ColorImage, Event, Pos2, UserData, ViewportCommand};

use crate::app::GuiApp;
use crate::camera::{Camera, between};
use crate::scene::Toggles;

/// The capture window's inner size, in points.
pub const WINDOW: [f32; 2] = [1600.0, 1000.0];
/// The card widths of the steps after the fit view, in points. 33 is the G1 review's case (zoom 0.199, unit `040`).
pub const WIDTHS: [f32; 7] = [16.0, 33.0, 40.0, 80.0, 130.0, 200.0, 400.0];
/// Frames per zoom sweep, and the sweeps.
pub const SWEEP_FRAMES: usize = 120;
pub const SWEEPS: usize = 2;
/// Frames drawn at a step before its screenshot, so text and layout settle.
const SETTLE: u32 = 2;
/// A step waits at most this many frames for its cards' details.
const DETAILS_WAIT: u32 = 900;
/// A screenshot request without a reply after this many frames is sent again. A focus change can drop one.
const RESEND: u32 = 30;
/// A capture that has not finished after this time stops with an error.
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// One camera step.
#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    /// The card width, or `None` for the fit view.
    pub width: Option<f32>,
    pub toggles: Toggles,
    /// The camera of a zoom to the selection, when there is one, in place of `width`.
    pub zoom_to: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Settle(usize, u32),
    /// The step, and the frames since its screenshot request.
    Await(usize, u32),
    Sweep {
        pass: usize,
        frame: usize,
    },
    Done,
}

pub struct Capture {
    pub dir: PathBuf,
    /// The world point the steps center on.
    pub target: Pos2,
    pub steps: Vec<Step>,
    phase: Phase,
    times: Vec<f32>,
    report: String,
    pub error: Option<String>,
    /// Frames that the current step waited for details.
    waited: u32,
}

/// The steps: the fit view, each card width of [`WIDTHS`], then card width 200 with each toggle off in turn,
/// card width 400 with every toggle on again, and the zoom to the selection.
pub fn steps() -> Vec<Step> {
    let all = Toggles::default();
    let step = |name: String, width, toggles| Step {
        name,
        width,
        toggles,
        zoom_to: false,
    };
    let mut out = vec![step("fit".into(), None, all)];
    out.extend(
        WIDTHS
            .iter()
            .map(|&w| step(format!("w{w:.0}"), Some(w), all)),
    );
    for (name, toggles) in [
        (
            "no-names",
            Toggles {
                names: false,
                ..all
            },
        ),
        (
            "no-shape",
            Toggles {
                shape: false,
                ..all
            },
        ),
    ] {
        out.push(step(format!("w200-{name}"), Some(200.0), toggles));
    }
    out.push(step("w400-all".into(), Some(400.0), all));
    out.push(Step {
        zoom_to: true,
        ..step("selection".into(), None, all)
    });
    out
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[k]
}

impl Capture {
    pub fn new(dir: PathBuf, target: Pos2, steps: Vec<Step>) -> Result<Capture> {
        fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
        Ok(Capture {
            dir,
            target,
            steps,
            phase: Phase::Settle(0, SETTLE),
            times: Vec::new(),
            report: String::new(),
            error: None,
            waited: 0,
        })
    }

    /// The camera of a step.
    pub fn camera(&self, app: &GuiApp, step: &Step) -> Camera {
        if step.zoom_to
            && let Some(e) = app.selection
        {
            return app.camera_for(e);
        }
        match step.width {
            None => app.bounds.fit(app.region),
            // A selected card shows as a zoom to it does, so a tall card shows its top.
            Some(w) if let Some(crate::scene::Entry::File(f)) = app.selection => {
                app.card_camera(f, w / app.map.card_w())
            }
            Some(w) => app.bounds.clamp(
                app.region,
                Camera {
                    center: self.target,
                    zoom: w / app.map.card_w(),
                },
            ),
        }
    }

    /// The sweep camera at a frame: from the fit view to card width 400 on the target.
    fn sweep_camera(&self, app: &GuiApp, frame: usize) -> Camera {
        let from = app.bounds.fit(app.region);
        let to = self.camera(
            app,
            &Step {
                name: String::new(),
                width: Some(400.0),
                toggles: Toggles::default(),
                zoom_to: false,
            },
        );
        let t = frame as f32 / (SWEEP_FRAMES - 1) as f32;
        app.bounds.clamp(app.region, between(from, to, t))
    }

    fn save(&self, name: &str, image: &ColorImage) -> Result<()> {
        let [w, h] = image.size;
        let bytes: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|c| c.to_srgba_unmultiplied())
            .collect();
        let img = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
            .context("the screenshot has the wrong size")?;
        let path = self.dir.join(format!("{name}.png"));
        img.save(&path)
            .with_context(|| format!("cannot write {}", path.display()))
    }

    /// Runs one frame of the script. `cpu` is the previous frame's CPU time in seconds.
    pub fn step(
        &mut self,
        app: &mut GuiApp,
        ctx: &egui::Context,
        events: &[Event],
        cpu: Option<f32>,
    ) {
        ctx.request_repaint();
        match self.phase {
            Phase::Settle(i, left) => {
                let cam = self.camera(app, &self.steps[i]);
                app.motion.jump(cam);
                app.opts.toggles = self.steps[i].toggles;
                app.update_details();
                if (app.details_missing() > 0 || app.panel_loading()) && self.waited < DETAILS_WAIT
                {
                    self.waited += 1;
                    return;
                }
                if left == 0 {
                    self.waited = 0;
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::new(i)));
                    self.phase = Phase::Await(i, 0);
                } else {
                    self.phase = Phase::Settle(i, left - 1);
                }
            }
            Phase::Await(i, waited) => {
                let shot = events.iter().find_map(|e| match e {
                    Event::Screenshot {
                        user_data, image, ..
                    } if user_data
                        .data
                        .as_ref()
                        .and_then(|d| d.downcast_ref::<usize>())
                        == Some(&i) =>
                    {
                        Some(image.clone())
                    }
                    _ => None,
                });
                let Some(image) = shot else {
                    self.phase = if waited >= RESEND {
                        Phase::Settle(i, 0)
                    } else {
                        Phase::Await(i, waited + 1)
                    };
                    return;
                };
                {
                    let name = format!("{:02}-{}", i + 1, self.steps[i].name);
                    if let Err(e) = self.save(&name, &image) {
                        self.error = Some(format!("{e:#}"));
                    }
                    self.phase = if i + 1 < self.steps.len() {
                        Phase::Settle(i + 1, SETTLE)
                    } else {
                        let _ = writeln!(
                            self.report,
                            "window {:.0} x {:.0} points, {} pixels per point; map {} x {} cells, {} cards",
                            WINDOW[0],
                            WINDOW[1],
                            ctx.pixels_per_point(),
                            app.map.layout.w,
                            app.map.layout.h,
                            app.map.layout.tiles.len()
                        );
                        app.opts.toggles = Toggles::default();
                        Phase::Sweep { pass: 0, frame: 0 }
                    };
                }
            }
            Phase::Sweep { pass, frame } => {
                // A frame's CPU time arrives with the next frame.
                if frame > 0
                    && let Some(s) = cpu
                {
                    self.times.push(s * 1000.0);
                }
                if frame == SWEEP_FRAMES {
                    self.end_pass(ctx, pass);
                    self.phase = if pass + 1 < SWEEPS {
                        Phase::Sweep {
                            pass: pass + 1,
                            frame: 0,
                        }
                    } else {
                        self.finish(ctx);
                        Phase::Done
                    };
                    return;
                }
                let cam = self.sweep_camera(app, frame);
                app.motion.jump(cam);
                self.phase = Phase::Sweep {
                    pass,
                    frame: frame + 1,
                };
            }
            Phase::Done => {}
        }
    }

    fn end_pass(&mut self, ctx: &egui::Context, pass: usize) {
        let mut t = std::mem::take(&mut self.times);
        t.sort_by(f32::total_cmp);
        let (size, fill) = ctx.fonts(|f| (f.font_image_size(), f.font_atlas_fill_ratio()));
        let _ = writeln!(
            self.report,
            "pass {}: {} frames, frame CPU p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms; font atlas {} x {}, {:.1} % full",
            pass + 1,
            t.len(),
            percentile(&t, 0.5),
            percentile(&t, 0.95),
            t.last().copied().unwrap_or(0.0),
            size[0],
            size[1],
            fill * 100.0
        );
    }

    fn finish(&mut self, ctx: &egui::Context) {
        let path = self.dir.join("frames.txt");
        if let Err(e) = fs::write(&path, &self.report) {
            self.error = Some(format!("cannot write {}: {e}", path.display()));
        }
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    pub fn report(&self) -> &str {
        &self.report
    }
}
