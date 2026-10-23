//! The icons of the primitives the shape list creates: cube, cylinder,
//! plane, sphere and the rest.

use super::Icon;
use super::pen::Pen;
use std::f32::consts::{PI, TAU};

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::Cube => {
            pen.polygon(&[
                (12.0, 2.5),
                (20.5, 7.0),
                (20.5, 17.0),
                (12.0, 21.5),
                (3.5, 17.0),
                (3.5, 7.0),
            ]);
            pen.polyline(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
            pen.polyline(&[(12.0, 11.5), (12.0, 21.5)]);
        }
        Icon::Cylinder => {
            pen.ellipse((12.0, 5.5), (8.0, 3.0));
            pen.polyline(&[(4.0, 5.5), (4.0, 18.5)]);
            pen.polyline(&[(20.0, 5.5), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
        }
        Icon::Plane => {
            pen.polygon(&[(2.0, 17.0), (8.0, 8.0), (22.0, 8.0), (16.0, 17.0)]);
        }
        Icon::Sphere => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.ellipse((12.0, 12.0), (9.0, 3.5));
        }
        Icon::Disc => {
            pen.ellipse((12.0, 12.0), (9.0, 4.5));
            pen.dot(12.0, 12.0, 1.2);
        }
        Icon::Tube => {
            pen.ellipse((12.0, 5.5), (8.0, 3.0));
            pen.ellipse((12.0, 5.5), (4.0, 1.5));
            pen.polyline(&[(4.0, 5.5), (4.0, 18.5)]);
            pen.polyline(&[(20.0, 5.5), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
        }
        Icon::Cone => {
            pen.polyline(&[(4.0, 18.5), (12.0, 3.0), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
            let back = pen.arc((12.0, 18.5), (8.0, 3.0), PI, TAU);
            pen.line(back);
        }
        Icon::Icosphere => {
            // A ball with its triangles: a hexagon of triangles round the
            // middle point.
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            let rim: Vec<(f32, f32)> = (0..6)
                .map(|k| {
                    let a = PI / 6.0 + k as f32 * PI / 3.0;
                    (12.0 + 9.0 * a.cos(), 12.0 + 9.0 * a.sin())
                })
                .collect();
            for (k, &(x, y)) in rim.iter().enumerate() {
                pen.polyline(&[(12.0, 12.0), (x, y)]);
                let (nx, ny) = rim[(k + 1) % rim.len()];
                pen.polyline(&[(x, y), (nx, ny)]);
            }
        }
        Icon::Torus => {
            pen.ellipse((12.0, 12.0), (10.0, 6.0));
            pen.ellipse((12.0, 11.0), (4.0, 1.8));
        }
        Icon::Capsule => {
            let mut outline = pen.arc((12.0, 8.0), (5.5, 5.5), PI, TAU);
            outline.extend(pen.arc((12.0, 16.0), (5.5, 5.5), 0.0, PI));
            let first = outline[0];
            outline.push(first);
            pen.line(outline);
        }
        Icon::Dome => {
            let top = pen.arc((12.0, 17.0), (9.0, 11.0), PI, TAU);
            pen.line(top);
            pen.ellipse((12.0, 17.0), (9.0, 2.5));
        }
        Icon::Elbow => {
            // A quarter bend from the bottom to the right side.
            let outer = pen.arc((20.0, 20.0), (15.0, 15.0), PI, 1.5 * PI);
            let inner = pen.arc((20.0, 20.0), (7.0, 7.0), PI, 1.5 * PI);
            pen.line(outer);
            pen.line(inner);
            pen.polyline(&[(5.0, 20.0), (13.0, 20.0)]);
            pen.polyline(&[(20.0, 5.0), (20.0, 13.0)]);
        }
        Icon::Sector => {
            let mut slice = vec![pen.p(5.0, 18.0)];
            slice.extend(pen.arc((5.0, 18.0), (15.0, 15.0), 1.5 * PI, TAU));
            slice.push(pen.p(5.0, 18.0));
            pen.line(slice);
        }
        Icon::Gear => {
            let teeth = 8;
            let mut outline: Vec<(f32, f32)> = Vec::new();
            for k in 0..teeth {
                let step = TAU / teeth as f32;
                let a = k as f32 * step;
                for (da, r) in [(0.0, 7.0), (0.15, 9.5), (0.35, 9.5), (0.5, 7.0)] {
                    let angle = a + da * step;
                    outline.push((12.0 + r * angle.cos(), 12.0 + r * angle.sin()));
                }
            }
            pen.polygon(&outline);
            pen.ellipse((12.0, 12.0), (2.5, 2.5));
        }
        Icon::HexGrid => {
            let hex = |cx: f32, cy: f32| -> Vec<(f32, f32)> {
                (0..6)
                    .map(|k| {
                        let a = k as f32 * PI / 3.0;
                        (cx + 4.5 * a.cos(), cy + 4.5 * a.sin())
                    })
                    .collect()
            };
            pen.polygon(&hex(7.0, 9.0));
            pen.polygon(&hex(13.75, 12.9));
            pen.polygon(&hex(7.0, 16.8));
        }
        Icon::HexSphere => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            let hex: Vec<(f32, f32)> = (0..6)
                .map(|k| {
                    let a = k as f32 * PI / 3.0;
                    (12.0 + 4.0 * a.cos(), 12.0 + 4.0 * a.sin())
                })
                .collect();
            pen.polygon(&hex);
            for &(x, y) in &hex {
                let (dx, dy) = (x - 12.0, y - 12.0);
                pen.polyline(&[(x, y), (12.0 + dx * 2.2, 12.0 + dy * 2.2)]);
            }
        }
        Icon::Stairs => {
            pen.polygon(&[
                (3.0, 21.0),
                (3.0, 17.0),
                (8.0, 17.0),
                (8.0, 12.0),
                (13.0, 12.0),
                (13.0, 7.0),
                (18.0, 7.0),
                (18.0, 3.0),
                (21.0, 3.0),
                (21.0, 21.0),
            ]);
        }
        Icon::Arch => {
            let mut outline = pen.points(&[(3.0, 21.0), (3.0, 12.0)]);
            outline.extend(pen.arc((12.0, 12.0), (9.0, 9.0), PI, TAU));
            outline.extend(pen.points(&[(21.0, 21.0), (16.0, 21.0), (16.0, 12.0)]));
            outline.extend(pen.arc((12.0, 12.0), (4.0, 4.0), 0.0, -PI));
            outline.extend(pen.points(&[(8.0, 21.0), (3.0, 21.0)]));
            pen.line(outline);
        }
        // Drawn by the other icon files.
        _ => {}
    }
}
