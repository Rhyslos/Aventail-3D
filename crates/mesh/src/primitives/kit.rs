//! Sizing primitives for a modular kit ([decision 0160](../../../../docs/decisions/0160-kit-helpers.md)):
//! pieces whose outside sizes are whole steps of a grid (1 m, 2 m, 4 m),
//! so they line up when placed side by side in a game engine.
//!
//! Two things are offered:
//!
//! - [`Primitive::scaled`]: every length times a factor, for making new
//!   shapes at the grid's size rather than at 1 m;
//! - [`Primitive::fitted_to_grid`]: the outside sizes rounded to the
//!   nearest whole number of grid steps, at least one. A size shorter than
//!   half a step is left alone (a wall's thickness, a slab's depth): it is
//!   not one the pieces line up along. Parts inside the outline (holes,
//!   teeth, a tube's wall) keep their proportion to it.
//!
//! Round shapes are fitted by how far they are across (their diameter),
//! the size that sits on the grid.

use super::*;

/// The smallest kit grid step, in centimetres (1 cm): anything finer is
/// no grid at all.
pub const MIN_GRID_STEP: f32 = 1.0;
/// The largest kit grid step, in centimetres (100 m).
pub const MAX_GRID_STEP: f32 = 10_000.0;

/// Whether `step` (centimetres) can be a kit grid: a real number from
/// [`MIN_GRID_STEP`] to [`MAX_GRID_STEP`].
pub fn is_grid_step(step: f32) -> bool {
    step.is_finite() && (MIN_GRID_STEP..=MAX_GRID_STEP).contains(&step)
}

/// `length` as a whole number of `step`s (at least one), or as it is when
/// shorter than half a step. Never past [`MAX_SIZE`].
fn fit(length: f32, step: f32) -> f32 {
    if length < step / 2.0 {
        return length;
    }
    let steps = (length / step).round().max(1.0);
    let fitted = steps * step;
    if fitted > MAX_SIZE {
        (MAX_SIZE / step).floor().max(1.0) * step
    } else {
        fitted
    }
}

/// How much a length must grow (or shrink) to be fitted: `fit(length) /
/// length`, 1 for nothing to fit.
fn ratio(length: f32, step: f32) -> f32 {
    if length > 0.0 {
        fit(length, step) / length
    } else {
        1.0
    }
}

impl Primitive {
    /// The same shape with every length (sizes, radii, heights, depths)
    /// times `factor`; counts and angles stay. A factor that isn't a
    /// positive number changes nothing. The result is within limits.
    #[must_use]
    pub fn scaled(&self, factor: f32) -> Self {
        let p = self.clamped();
        if !(factor.is_finite() && factor > 0.0) {
            return p;
        }
        let k = factor;
        let out = match p {
            Self::Cube(c) => Self::Cube(CubeParams {
                size: c.size * k,
                ..c
            }),
            Self::Plane(c) => Self::Plane(PlaneParams {
                width: c.width * k,
                depth: c.depth * k,
                ..c
            }),
            Self::Cylinder(c) => Self::Cylinder(CylinderParams {
                radius: c.radius * k,
                height: c.height * k,
                ..c
            }),
            Self::Sphere(c) => Self::Sphere(SphereParams {
                radius: c.radius * k,
                ..c
            }),
            Self::Disc(c) => Self::Disc(DiscParams {
                radius: c.radius * k,
                inner_radius: c.inner_radius * k,
                ..c
            }),
            Self::Tube(c) => Self::Tube(TubeParams {
                outer_radius: c.outer_radius * k,
                inner_radius: c.inner_radius * k,
                height: c.height * k,
                ..c
            }),
            Self::Cone(c) => Self::Cone(ConeParams {
                bottom_radius: c.bottom_radius * k,
                top_radius: c.top_radius * k,
                height: c.height * k,
                ..c
            }),
            Self::Icosphere(c) => Self::Icosphere(IcosphereParams {
                radius: c.radius * k,
                ..c
            }),
            Self::Torus(c) => Self::Torus(TorusParams {
                radius: c.radius * k,
                tube_radius: c.tube_radius * k,
                ..c
            }),
            Self::Capsule(c) => Self::Capsule(CapsuleParams {
                radius: c.radius * k,
                height: c.height * k,
                ..c
            }),
            Self::Dome(c) => Self::Dome(DomeParams {
                radius: c.radius * k,
                ..c
            }),
            Self::Elbow(c) => Self::Elbow(ElbowParams {
                bend_radius: c.bend_radius * k,
                pipe_radius: c.pipe_radius * k,
                inner_radius: c.inner_radius * k,
                ..c
            }),
            Self::Sector(c) => Self::Sector(SectorParams {
                radius: c.radius * k,
                inner_radius: c.inner_radius * k,
                height: c.height * k,
                ..c
            }),
            Self::Gear(c) => Self::Gear(GearParams {
                radius: c.radius * k,
                tooth_depth: c.tooth_depth * k,
                height: c.height * k,
                hole_radius: c.hole_radius * k,
                ..c
            }),
            Self::HexGrid(c) => Self::HexGrid(HexGridParams {
                size: c.size * k,
                ..c
            }),
            Self::HexSphere(c) => Self::HexSphere(HexSphereParams {
                radius: c.radius * k,
                ..c
            }),
            Self::Stairs(c) => Self::Stairs(StairsParams {
                width: c.width * k,
                height: c.height * k,
                depth: c.depth * k,
                ..c
            }),
            Self::Arch(c) => Self::Arch(ArchParams {
                width: c.width * k,
                leg_height: c.leg_height * k,
                thickness: c.thickness * k,
                depth: c.depth * k,
                ..c
            }),
        };
        out.clamped()
    }

    /// The same shape with its outside sizes a whole number of `step`s
    /// (centimetres; at least one step), as the module explains. A step
    /// that isn't a kit grid ([`is_grid_step`]) changes nothing. The
    /// result is within limits.
    #[must_use]
    pub fn fitted_to_grid(&self, step: f32) -> Self {
        let p = self.clamped();
        if !is_grid_step(step) {
            return p;
        }
        let f = |length: f32| fit(length, step);
        // A round shape's radius, fitted by its diameter.
        let across = |radius: f32| fit(radius * 2.0, step) / 2.0;
        let out = match p {
            Self::Cube(c) => Self::Cube(CubeParams {
                size: Vec3::new(f(c.size.x), f(c.size.y), f(c.size.z)),
                ..c
            }),
            Self::Plane(c) => Self::Plane(PlaneParams {
                width: f(c.width),
                depth: f(c.depth),
                ..c
            }),
            Self::Cylinder(c) => Self::Cylinder(CylinderParams {
                radius: across(c.radius),
                height: f(c.height),
                ..c
            }),
            Self::Sphere(c) => Self::Sphere(SphereParams {
                radius: across(c.radius),
                ..c
            }),
            Self::Icosphere(c) => Self::Icosphere(IcosphereParams {
                radius: across(c.radius),
                ..c
            }),
            Self::HexSphere(c) => Self::HexSphere(HexSphereParams {
                radius: across(c.radius),
                ..c
            }),
            Self::Dome(c) => Self::Dome(DomeParams {
                radius: across(c.radius),
                ..c
            }),
            Self::Disc(c) => {
                let k = ratio(c.radius * 2.0, step);
                Self::Disc(DiscParams {
                    radius: c.radius * k,
                    inner_radius: c.inner_radius * k,
                    ..c
                })
            }
            Self::Tube(c) => {
                let k = ratio(c.outer_radius * 2.0, step);
                Self::Tube(TubeParams {
                    outer_radius: c.outer_radius * k,
                    inner_radius: c.inner_radius * k,
                    height: f(c.height),
                    ..c
                })
            }
            Self::Cone(c) => {
                let k = ratio(c.bottom_radius.max(c.top_radius) * 2.0, step);
                Self::Cone(ConeParams {
                    bottom_radius: c.bottom_radius * k,
                    top_radius: c.top_radius * k,
                    height: f(c.height),
                    ..c
                })
            }
            Self::Torus(c) => {
                let k = ratio((c.radius + c.tube_radius) * 2.0, step);
                Self::Torus(TorusParams {
                    radius: c.radius * k,
                    tube_radius: c.tube_radius * k,
                    ..c
                })
            }
            Self::Capsule(c) => {
                let radius = across(c.radius);
                Self::Capsule(CapsuleParams {
                    radius,
                    height: f(c.height.max(radius * 2.0)).max(radius * 2.0),
                    ..c
                })
            }
            Self::Elbow(c) => {
                let k = ratio(c.bend_radius + c.pipe_radius, step);
                Self::Elbow(ElbowParams {
                    bend_radius: c.bend_radius * k,
                    pipe_radius: c.pipe_radius * k,
                    inner_radius: c.inner_radius * k,
                    ..c
                })
            }
            Self::Sector(c) => {
                let k = ratio(c.radius, step);
                Self::Sector(SectorParams {
                    radius: c.radius * k,
                    inner_radius: c.inner_radius * k,
                    height: f(c.height),
                    ..c
                })
            }
            Self::Gear(c) => {
                let k = ratio(c.radius * 2.0, step);
                Self::Gear(GearParams {
                    radius: c.radius * k,
                    tooth_depth: c.tooth_depth * k,
                    hole_radius: c.hole_radius * k,
                    height: f(c.height),
                    ..c
                })
            }
            Self::HexGrid(c) => {
                let b = p.bounds();
                let k = ratio(b.max.x - b.min.x, step);
                Self::HexGrid(HexGridParams {
                    size: c.size * k,
                    ..c
                })
            }
            Self::Stairs(c) => Self::Stairs(StairsParams {
                width: f(c.width),
                height: f(c.height),
                depth: f(c.depth),
                ..c
            }),
            Self::Arch(c) => fitted_arch(c, step),
        };
        out.clamped()
    }
}

/// An arch fitted: its outside width (the opening and both sides) and its
/// height to the top of the round, each a whole number of steps; the
/// round top's height comes from the width, so the legs take up the rest.
fn fitted_arch(c: ArchParams, step: f32) -> Primitive {
    let k = ratio(c.width + 2.0 * c.thickness, step);
    let (width, thickness) = (c.width * k, c.thickness * k);
    let round = width / 2.0 + thickness;
    let mut height = fit(c.leg_height + round, step);
    // The legs can't be shorter than nothing: a step taller instead.
    while height < round && height + step <= MAX_SIZE {
        height += step;
    }
    Primitive::Arch(ArchParams {
        width,
        thickness,
        leg_height: (height - round).max(0.0),
        depth: fit(c.depth, step),
        ..c
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `length` is a whole number of `step`s, give or take float
    /// rounding.
    fn whole(length: f32, step: f32) -> bool {
        let n = length / step;
        n >= 0.999 && (n - n.round()).abs() < 1e-3
    }

    #[test]
    fn a_length_rounds_to_the_nearest_whole_step_or_stays_when_thin() {
        assert_eq!(fit(100.0, 100.0), 100.0);
        assert_eq!(fit(140.0, 100.0), 100.0);
        assert_eq!(fit(160.0, 100.0), 200.0);
        assert_eq!(fit(50.0, 100.0), 100.0, "half a step rounds up");
        assert_eq!(fit(20.0, 100.0), 20.0, "a wall's thickness stays");
        assert_eq!(fit(100.0, 200.0), 200.0);
        assert_eq!(fit(100.0, 400.0), 100.0);
        assert_eq!(fit(130.0, 37.0), 148.0);
        // Never past the largest size.
        assert!(fit(MAX_SIZE, 300.0) <= MAX_SIZE);
        assert!(whole(fit(MAX_SIZE, 300.0), 300.0));
    }

    #[test]
    fn grid_steps_are_checked() {
        for good in [MIN_GRID_STEP, 100.0, 200.0, 400.0, 37.5, MAX_GRID_STEP] {
            assert!(is_grid_step(good), "{good}");
        }
        for bad in [0.0, -100.0, 0.5, f32::NAN, f32::INFINITY, 1e9] {
            assert!(!is_grid_step(bad), "{bad}");
        }
    }

    #[test]
    fn a_wall_piece_keeps_its_thickness_and_fits_its_length_and_height() {
        let wall = Primitive::Cube(CubeParams {
            size: Vec3::new(390.0, 20.0, 310.0),
            segments: [2, 1, 3],
        });
        let Primitive::Cube(c) = wall.fitted_to_grid(100.0) else {
            panic!("a cube")
        };
        assert_eq!(c.size, Vec3::new(400.0, 20.0, 300.0));
        assert_eq!(c.segments, [2, 1, 3], "counts stay");
    }

    #[test]
    fn every_kind_fits_its_outside_sizes_to_the_grid() {
        for step in [100.0, 200.0, 400.0, 37.0] {
            for shape in Primitive::all() {
                // Grown so every main size is past half a step.
                let shape = shape.scaled(step / 100.0 * 1.3);
                let fitted = shape.fitted_to_grid(step);
                assert_eq!(fitted.kind(), shape.kind());
                let ok = match fitted {
                    Primitive::Cube(c) => c.size.to_array().iter().all(|s| whole(*s, step)),
                    Primitive::Plane(c) => whole(c.width, step) && whole(c.depth, step),
                    Primitive::Cylinder(c) => whole(c.radius * 2.0, step) && whole(c.height, step),
                    Primitive::Sphere(c) => whole(c.radius * 2.0, step),
                    Primitive::Icosphere(c) => whole(c.radius * 2.0, step),
                    Primitive::HexSphere(c) => whole(c.radius * 2.0, step),
                    Primitive::Dome(c) => whole(c.radius * 2.0, step),
                    Primitive::Disc(c) => whole(c.radius * 2.0, step),
                    Primitive::Tube(c) => {
                        whole(c.outer_radius * 2.0, step) && whole(c.height, step)
                    }
                    Primitive::Cone(c) => {
                        whole(c.bottom_radius.max(c.top_radius) * 2.0, step)
                            && whole(c.height, step)
                    }
                    Primitive::Torus(c) => whole((c.radius + c.tube_radius) * 2.0, step),
                    Primitive::Capsule(c) => whole(c.radius * 2.0, step) && whole(c.height, step),
                    Primitive::Elbow(c) => whole(c.bend_radius + c.pipe_radius, step),
                    // The slab's thickness is thin: only the radius fits.
                    Primitive::Sector(c) => whole(c.radius, step),
                    Primitive::Gear(c) => whole(c.radius * 2.0, step),
                    Primitive::HexGrid(_) => {
                        let b = fitted.bounds();
                        whole(b.max.x - b.min.x, step)
                    }
                    Primitive::Stairs(c) => {
                        whole(c.width, step) && whole(c.height, step) && whole(c.depth, step)
                    }
                    Primitive::Arch(c) => {
                        whole(c.width + 2.0 * c.thickness, step)
                            && whole(c.leg_height + c.width / 2.0 + c.thickness, step)
                    }
                };
                assert!(ok, "{step}: {fitted:?}");
                // Fitting again changes nothing.
                let again = fitted.fitted_to_grid(step);
                assert_eq!(again.kind(), fitted.kind());
                let (a, b) = (again.bounds(), fitted.bounds());
                assert!(
                    (a.max - b.max).abs().max_element() < 0.01
                        && (a.min - b.min).abs().max_element() < 0.01,
                    "{step}: {fitted:?} then {again:?}"
                );
                // It still builds a sound mesh.
                let mesh = fitted.mesh();
                assert!(mesh.face_count() > 0);
                assert!(mesh.vertices().all(|(_, p)| p.is_finite()));
            }
        }
    }

    #[test]
    fn holes_and_teeth_keep_their_proportion() {
        let tube = Primitive::Tube(TubeParams {
            outer_radius: 60.0,
            inner_radius: 30.0,
            ..TubeParams::default()
        });
        let Primitive::Tube(t) = tube.fitted_to_grid(100.0) else {
            panic!("a tube")
        };
        assert_eq!(t.outer_radius, 50.0);
        assert_eq!(t.inner_radius, 25.0);
        let gear = Primitive::Gear(GearParams {
            radius: 75.0,
            ..GearParams::default()
        });
        let Primitive::Gear(g) = gear.fitted_to_grid(100.0) else {
            panic!("a gear")
        };
        assert_eq!(g.radius, 100.0);
        assert!((g.tooth_depth / g.radius - 8.0 / 75.0).abs() < 1e-5);
        assert_eq!(g.height, 10.0, "a thin slab stays thin");
    }

    #[test]
    fn an_arch_too_short_for_its_round_top_grows_a_step() {
        let arch = Primitive::Arch(ArchParams {
            width: 300.0,
            leg_height: 0.0,
            thickness: 50.0,
            depth: 30.0,
            segments: 12,
        });
        let Primitive::Arch(a) = arch.fitted_to_grid(100.0) else {
            panic!("an arch")
        };
        assert_eq!(a.width + 2.0 * a.thickness, 400.0);
        assert_eq!(a.leg_height + a.width / 2.0 + a.thickness, 200.0);
        assert!(a.leg_height >= 0.0);
        // The tallest possible arch stays within limits.
        let huge = Primitive::Arch(ArchParams {
            width: MAX_SIZE,
            leg_height: MAX_SIZE,
            thickness: MAX_SIZE,
            depth: MAX_SIZE,
            segments: 12,
        })
        .fitted_to_grid(MAX_GRID_STEP);
        assert_eq!(huge, huge.clamped());
    }

    #[test]
    fn silly_steps_and_factors_change_nothing_and_never_panic() {
        for shape in Primitive::all() {
            for step in [0.0, -1.0, f32::NAN, f32::INFINITY, 1e12] {
                assert_eq!(shape.fitted_to_grid(step), shape.clamped(), "{step}");
            }
            for factor in [0.0, -2.0, f32::NAN, f32::INFINITY] {
                assert_eq!(shape.scaled(factor), shape.clamped(), "{factor}");
            }
            // Huge and tiny factors are brought within limits.
            for factor in [1e9, 1e-9] {
                let s = shape.scaled(factor);
                assert_eq!(s, s.clamped());
            }
            // Tiny shapes fitted to the finest grid stay as they are or
            // grow to a step; never NaN.
            let tiny = shape.scaled(1e-9).fitted_to_grid(MIN_GRID_STEP);
            assert!(tiny.mesh().vertices().all(|(_, p)| p.is_finite()));
        }
    }

    #[test]
    fn scaling_by_two_doubles_the_box() {
        for shape in Primitive::all() {
            let (a, b) = (shape.bounds(), shape.scaled(2.0).bounds());
            assert!(
                ((b.max - b.min) - (a.max - a.min) * 2.0)
                    .abs()
                    .max_element()
                    < 0.05,
                "{shape:?}"
            );
        }
    }
}
