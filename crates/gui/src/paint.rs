//! Paints draw items. It reads only the items, so every decision about what shows is in `scene`.
//!
//! Text lays out at its glyph size on the ladder, and its mesh scales to its exact size (unit `040`),
//! so text grows and shrinks smoothly with the zoom while the glyph atlas holds only ladder sizes.

use std::sync::Arc;

use eframe::egui::{
    Align2, Color32, Galley, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, TextureId, Vec2,
    pos2, vec2,
};

use crate::fonts::font;
use crate::scene::{Body, Item};

/// A square icon is this share of the icon size.
const SQUARE: f32 = 0.6;

pub fn items(painter: &Painter, items: &[Item]) {
    // Lay out every text first, so the font atlas has every glyph of the frame when the meshes read its size.
    let galleys: Vec<Option<Arc<Galley>>> = items
        .iter()
        .map(|item| {
            let text = match &item.body {
                Body::Str(s) => s.clone(),
                Body::Glyph(c) => c.to_string(),
                _ => return None,
            };
            let color = Color32::from_rgb(item.color.r(), item.color.g(), item.color.b());
            Some(painter.layout_no_wrap(text, font(item.glyph, item.bold), color))
        })
        .collect();
    let atlas = painter.ctx().fonts(|f| f.font_image_size());
    for (item, galley) in items.iter().zip(galleys) {
        match &item.body {
            Body::Fill => {
                painter.rect_filled(item.rect, item.radius, item.color);
            }
            Body::Stroke(w) => {
                painter.rect_stroke(
                    item.rect,
                    item.radius,
                    Stroke::new(*w, item.color),
                    StrokeKind::Outside,
                );
            }
            Body::Bars(bars) => {
                // One mesh of plain quads: a shape can have a bar per screen row, and rects with feathering cost more.
                let mut mesh = Mesh::default();
                for b in bars {
                    mesh.add_colored_rect(*b, item.color);
                }
                painter.add(Shape::mesh(mesh));
            }
            Body::Halo { inner, filled } => {
                painter.add(Shape::mesh(halo(*inner, item.rect, *filled, item.color)));
            }
            Body::Dot => {
                painter.circle_filled(item.rect.center(), item.rect.width() / 2.0, item.color);
            }
            Body::Square => {
                let r = Rect::from_center_size(item.rect.center(), Vec2::splat(SQUARE * item.size));
                painter.rect_filled(r, 1.0, item.color);
            }
            Body::Greek => {
                painter.rect_filled(
                    item.rect,
                    item.radius,
                    item.color.gamma_multiply(item.opacity),
                );
            }
            Body::Glyph(_) | Body::Str(_) => {
                let Some(galley) = galley else { continue };
                let anchor = if matches!(item.body, Body::Glyph(_)) {
                    Align2::CENTER_CENTER
                } else {
                    Align2::LEFT_CENTER
                };
                let at = match anchor {
                    Align2::CENTER_CENTER => item.rect.center(),
                    _ => item.rect.left_center(),
                };
                painter.add(Shape::mesh(scaled_text(
                    &galley,
                    at,
                    anchor,
                    item.size / item.glyph,
                    item.color,
                    item.opacity,
                    atlas,
                )));
            }
        }
    }
}

/// A glow's halo (unit `049`) as one mesh: the color at `inner`'s edge, fading to nothing at `outer`'s edge, and inside `inner` when `filled`.
fn halo(inner: Rect, outer: Rect, filled: bool, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let corners = |r: Rect| {
        [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ]
    };
    for p in corners(inner) {
        mesh.colored_vertex(p, color);
    }
    for p in corners(outer) {
        mesh.colored_vertex(p, Color32::TRANSPARENT);
    }
    // One quad per side, between the inner and the outer corners.
    for k in 0..4u32 {
        let j = (k + 1) % 4;
        mesh.add_triangle(k, j, 4 + j);
        mesh.add_triangle(k, 4 + j, 4 + k);
    }
    if filled {
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
    }
    mesh
}

/// A galley's glyphs as one mesh, scaled by `k` around its anchor point, with the font atlas's texel coordinates normalized,
/// as egui's tessellator does for text.
fn scaled_text(
    galley: &Galley,
    at: Pos2,
    anchor: Align2,
    k: f32,
    color: Color32,
    opacity: f32,
    atlas: [usize; 2],
) -> Mesh {
    let size = galley.size() * k;
    let min = anchor.anchor_size(at, size).min;
    let uv = vec2(1.0 / atlas[0] as f32, 1.0 / atlas[1] as f32);
    let mut mesh = Mesh::with_texture(TextureId::default());
    for row in &galley.rows {
        let base = mesh.vertices.len() as u32;
        let mesh_row = &row.visuals.mesh;
        mesh.indices
            .extend(mesh_row.indices.iter().map(|i| i + base));
        mesh.vertices.extend(mesh_row.vertices.iter().map(|v| {
            let c = if v.color == Color32::PLACEHOLDER {
                color
            } else {
                v.color
            };
            eframe::epaint::Vertex {
                pos: min + (row.pos.to_vec2() + v.pos.to_vec2()) * k,
                uv: pos2(v.uv.x * uv.x, v.uv.y * uv.y),
                color: if opacity < 1.0 {
                    c.gamma_multiply(opacity)
                } else {
                    c
                },
            }
        }));
    }
    mesh
}
