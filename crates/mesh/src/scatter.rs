//! Scattering shapes over a surface ([decision 0146](../../../docs/decisions/0146-scatter.md)):
//! copies of one or more shapes placed on a surface's points, on its
//! polygons' centres, or at random over it, each standing on the surface
//! (turned to face out from it), with a random turn and size if asked.
//!
//! **Where the copies go** ([`PlaceOn`]):
//!
//! - **Points:** one copy on each point of the surface, facing the way the
//!   polygons round the point face on average.
//! - **Polygon centres:** one on the middle of each polygon, facing the way
//!   the polygon does.
//! - **Random:** [`ScatterOptions::count`] copies spread evenly by area: a
//!   polygon twice the size gets twice the copies, on average.
//!
//! **Spacing** keeps copies at least that far apart (centre to centre):
//! a spot too close to one already taken is skipped. Over a random spread
//! the app tries again, up to thirty tries a copy, so a spacing too large
//! for the surface gives fewer copies rather than none.
//!
//! **Each copy** is the shape as it stands (its own turn and size kept),
//! moved so its pivot sits on the spot. With *Align to surface* on, the
//! shape's up (Z) is turned onto the surface's outward direction, as a
//! rock stands on a slope. Then *Random turn* turns it round that
//! direction, *Random scale* sizes it, and *Offset* lifts it along the
//! surface's direction (or sinks it, below 0).
//!
//! **Deterministic:** the same surface, shapes and settings always give
//! the same copies. A copy's turn, size and shape are worked out from the
//! seed and the point or polygon it stands on (its id), or its place in
//! the random spread, so raising Count adds copies without moving the
//! others, and editing one part of the surface leaves the copies on the
//! rest where they were.
//!
//! The result is the surface with every copy added as polygons of its own
//! (new ids, after the surface's, which keep theirs): one mesh, as exports
//! need. UVs, hard edges, creases and materials come along with each copy.

use crate::parts::append;
use crate::{FaceId, Mesh, VertexId};
use glam::{Mat4, Quat, Vec3};
use std::collections::{BTreeSet, HashMap};
use words::{tr, trf};

/// Where the copies go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlaceOn {
    /// On each point of the surface.
    Points,
    /// On the middle of each polygon.
    Centres,
    /// At random over the surface, evenly by area: [`ScatterOptions::count`]
    /// of them.
    #[default]
    Random,
}

impl PlaceOn {
    /// Every choice, in the order the interface lists them.
    pub const ALL: [Self; 3] = [Self::Points, Self::Centres, Self::Random];
}

/// The settings of a scatter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterOptions {
    /// Where the copies go.
    pub place_on: PlaceOn,
    /// How many copies a random spread makes (the other ways make one per
    /// point or polygon).
    pub count: u32,
    /// The seed every random choice starts from: another seed, another
    /// spread, turns and sizes.
    pub seed: u32,
    /// The least distance between two copies' spots, in centimetres: 0
    /// for no limit.
    pub spacing: f32,
    /// Whether each copy stands on the surface, its up turned to the
    /// surface's outward direction. Off, copies keep the shape's own way
    /// up.
    pub align: bool,
    /// The most a copy is turned round its up direction at random, in
    /// degrees: 0 turns none, 360 any way at all.
    pub random_turn: f32,
    /// The smallest size a copy is given at random, as a share of the
    /// shape's (1 is as it is).
    pub scale_min: f32,
    /// The largest size a copy is given at random, as a share.
    pub scale_max: f32,
    /// How far each copy is moved out along the surface's direction, in
    /// centimetres (below 0, sunk in).
    pub offset: f32,
}

impl ScatterOptions {
    /// The standard count for a random spread.
    pub const STANDARD_COUNT: u32 = 50;
    /// The most copies a scatter can make.
    pub const MAX_COPIES: u32 = 100_000;
    /// The largest spacing and offset: 1 km.
    pub const MAX_DISTANCE: f32 = 100_000.0;
    /// The smallest random size: 1%.
    pub const MIN_SCALE: f32 = 0.01;
    /// The largest random size: 100 times.
    pub const MAX_SCALE: f32 = 100.0;
    /// How many tries a random spread makes for each copy when spacing
    /// turns spots down.
    const TRIES_PER_COPY: u64 = 30;

    /// The settings brought within limits: values that aren't real numbers
    /// become the standard ones, and a smallest size above the largest is
    /// swapped with it.
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let real = |value: f32, otherwise: f32| if value.is_finite() { value } else { otherwise };
        let scale = |value: f32| real(value, 1.0).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
        let (low, high) = (scale(self.scale_min), scale(self.scale_max));
        Self {
            place_on: self.place_on,
            count: self.count.clamp(1, Self::MAX_COPIES),
            seed: self.seed,
            spacing: real(self.spacing, standard.spacing).clamp(0.0, Self::MAX_DISTANCE),
            align: self.align,
            random_turn: real(self.random_turn, standard.random_turn).clamp(0.0, 360.0),
            scale_min: low.min(high),
            scale_max: low.max(high),
            offset: real(self.offset, standard.offset)
                .clamp(-Self::MAX_DISTANCE, Self::MAX_DISTANCE),
        }
    }
}

impl Default for ScatterOptions {
    fn default() -> Self {
        Self {
            place_on: PlaceOn::Random,
            count: Self::STANDARD_COUNT,
            seed: 1,
            spacing: 0.0,
            align: true,
            random_turn: 0.0,
            scale_min: 1.0,
            scale_max: 1.0,
            offset: 0.0,
        }
    }
}

/// One shape to scatter.
#[derive(Debug, Clone, Copy)]
pub struct ScatterShape<'a> {
    /// The shape.
    pub mesh: &'a Mesh,
    /// From the shape's space into the surface's, with the shape's pivot
    /// (the spot it stands on) at the surface's origin.
    pub matrix: Mat4,
}

/// Why a scatter can't be made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScatterProblem {
    /// No shapes to scatter.
    NoShapes,
    /// The surface has no polygons (or none of the chosen ones).
    NoSurface,
    /// The chosen polygons aren't on the surface any more.
    ChosenGone,
    /// The surface has no area to spread copies over.
    NoArea,
    /// More copies than [`ScatterOptions::MAX_COPIES`].
    TooManyCopies {
        /// The copies it would make.
        copies: usize,
    },
    /// The result would have more polygons than allowed.
    TooManyPolygons {
        /// The polygons it would have.
        polygons: usize,
        /// The most allowed.
        most: usize,
    },
}

impl std::fmt::Display for ScatterProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoShapes => f.write_str(tr("there are no shapes to scatter")),
            Self::NoSurface => f.write_str(tr("the surface has no polygons to scatter onto")),
            Self::ChosenGone => f.write_str(tr(
                "none of the polygons it was limited to are on the shape any more: choose them \
                 again, or use the whole surface",
            )),
            Self::NoArea => f.write_str(tr("the surface has no area to spread copies over")),
            Self::TooManyCopies { copies } => f.write_str(&trf!(
                "it would make {} copies, and the most is 100 000: use Random, or a larger \
                 Spacing",
                copies
            )),
            Self::TooManyPolygons { polygons, most } => f.write_str(&trf!(
                "the result would have {} polygons, and the most is {}: fewer copies, or a \
                 simpler shape",
                polygons,
                most
            )),
        }
    }
}

impl std::error::Error for ScatterProblem {}

/// One copy's spot: where it stands, which way is out, its turn and size,
/// and which shape it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// Where its pivot goes, before the offset.
    pub position: Vec3,
    /// The surface's outward direction there (unit length).
    pub normal: Vec3,
    /// Its random turn round its up direction, in radians.
    pub turn: f32,
    /// Its random size, as a share.
    pub scale: f32,
    /// Which of the shapes it is (an index into them).
    pub shape: usize,
}

impl Spot {
    /// The matrix placing a shape (pivot at the origin) at this spot.
    pub fn matrix(&self, options: &ScatterOptions) -> Mat4 {
        // The shape's up (Z) turned onto the surface's direction; the
        // random turn is round the shape's own up, so round the surface's
        // direction once stood on it.
        let stand = if options.align {
            Quat::from_rotation_arc(Vec3::Z, self.normal)
        } else {
            Quat::IDENTITY
        };
        let turn = Quat::from_axis_angle(Vec3::Z, self.turn);
        // The offset follows the surface's direction, aligned or not.
        Mat4::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            stand * turn,
            self.position + self.normal * options.offset,
        )
    }
}

/// A small deterministic random number source (SplitMix64): the same
/// numbers on every computer.
#[derive(Debug, Clone)]
struct Random(u64);

impl Random {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number from 0 up to (not including) 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// The random turn, size and shape of the copy keyed `key` (a point or
/// polygon id, or a place in the random spread).
fn looks(options: &ScatterOptions, shapes: usize, kind: u64, key: u64) -> (f32, f32, usize) {
    let mut random = Random::new(crate::names::mix(
        crate::names::mix(u64::from(options.seed), kind),
        key,
    ));
    let turn = random.unit() * options.random_turn.to_radians();
    let scale = options.scale_min + random.unit() * (options.scale_max - options.scale_min);
    let shape = if shapes <= 1 {
        0
    } else {
        (random.next() % shapes as u64) as usize
    };
    (turn, scale, shape)
}

/// Remembers the spots taken, to keep new ones `spacing` away.
struct Spacing {
    spacing: f32,
    cells: HashMap<[i64; 3], Vec<Vec3>>,
}

impl Spacing {
    fn new(spacing: f32) -> Self {
        Self {
            spacing,
            cells: HashMap::new(),
        }
    }

    fn cell(&self, p: Vec3) -> [i64; 3] {
        // Saturating: a far place lands in an edge cell, still correct.
        let at = |x: f32| (x / self.spacing).floor() as i64;
        [at(p.x), at(p.y), at(p.z)]
    }

    /// Takes `p` if it is far enough from every spot taken.
    fn take(&mut self, p: Vec3) -> bool {
        if self.spacing <= 0.0 {
            return true;
        }
        let [x, y, z] = self.cell(p);
        let squared = self.spacing * self.spacing;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let key = [
                        x.saturating_add(dx),
                        y.saturating_add(dy),
                        z.saturating_add(dz),
                    ];
                    if let Some(taken) = self.cells.get(&key)
                        && taken.iter().any(|q| q.distance_squared(p) < squared)
                    {
                        return false;
                    }
                }
            }
        }
        self.cells.entry([x, y, z]).or_default().push(p);
        true
    }
}

/// Keys telling the three ways' random streams apart.
const POINT_KEY: u64 = 0x5ca7_0001;
const CENTRE_KEY: u64 = 0x5ca7_0002;
const RANDOM_KEY: u64 = 0x5ca7_0003;

/// The polygons copies go on: those of `only` on the surface, or all.
fn surface_faces(
    surface: &Mesh,
    only: Option<&BTreeSet<FaceId>>,
) -> Result<Vec<FaceId>, ScatterProblem> {
    let all: Vec<FaceId> = surface.faces().map(|(f, _)| f).collect();
    if all.is_empty() {
        return Err(ScatterProblem::NoSurface);
    }
    let Some(only) = only else {
        return Ok(all);
    };
    let kept: Vec<FaceId> = all.into_iter().filter(|f| only.contains(f)).collect();
    if kept.is_empty() {
        return Err(ScatterProblem::ChosenGone);
    }
    Ok(kept)
}

/// Whether every corner of `corners` is at a real position.
fn real_polygon(surface: &Mesh, corners: &[VertexId]) -> bool {
    corners
        .iter()
        .all(|v| surface.position(*v).is_some_and(|p| p.is_finite()))
}

/// The spots `options` places copies of `shapes` (how many shapes) on,
/// over `surface` (only the polygons in `only`, when given). Points or
/// polygons that aren't at real positions are skipped.
pub fn spots(
    surface: &Mesh,
    only: Option<&BTreeSet<FaceId>>,
    options: &ScatterOptions,
    shapes: usize,
) -> Result<Vec<Spot>, ScatterProblem> {
    let options = options.clamped();
    if shapes == 0 {
        return Err(ScatterProblem::NoShapes);
    }
    let faces = surface_faces(surface, only)?;
    let mut spacing = Spacing::new(options.spacing);
    let mut out = Vec::new();
    let mut push = |out: &mut Vec<Spot>, position: Vec3, normal: Vec3, kind: u64, key: u64| {
        if !spacing.take(position) {
            return;
        }
        let (turn, scale, shape) = looks(&options, shapes, kind, key);
        out.push(Spot {
            position,
            normal,
            turn,
            scale,
            shape,
        });
    };
    match options.place_on {
        PlaceOn::Points => {
            // Each point's direction: the sum of its polygons' (those
            // copies go on).
            let mut normals: std::collections::BTreeMap<VertexId, Vec3> = Default::default();
            for f in &faces {
                let Some(corners) = surface.face(*f) else {
                    continue;
                };
                if !real_polygon(surface, corners) {
                    continue;
                }
                let normal = surface.face_normal(*f);
                for v in corners {
                    *normals.entry(*v).or_insert(Vec3::ZERO) += normal;
                }
            }
            if normals.len() > ScatterOptions::MAX_COPIES as usize {
                return Err(ScatterProblem::TooManyCopies {
                    copies: normals.len(),
                });
            }
            for (v, sum) in normals {
                let Some(position) = surface.position(v) else {
                    continue;
                };
                let normal = sum.try_normalize().unwrap_or(Vec3::Z);
                push(&mut out, position, normal, POINT_KEY, u64::from(v.get()));
            }
        }
        PlaceOn::Centres => {
            if faces.len() > ScatterOptions::MAX_COPIES as usize {
                return Err(ScatterProblem::TooManyCopies {
                    copies: faces.len(),
                });
            }
            for f in &faces {
                let Some(corners) = surface.face(*f) else {
                    continue;
                };
                if corners.is_empty() || !real_polygon(surface, corners) {
                    continue;
                }
                let sum: Vec3 = corners.iter().filter_map(|v| surface.position(*v)).sum();
                let centre = sum / corners.len() as f32;
                push(
                    &mut out,
                    centre,
                    surface.face_normal(*f),
                    CENTRE_KEY,
                    u64::from(f.get()),
                );
            }
        }
        PlaceOn::Random => {
            // Every triangle with its polygon's direction, and the running
            // total of their areas, to pick one by area.
            let mut triangles: Vec<([Vec3; 3], Vec3)> = Vec::new();
            let mut totals: Vec<f64> = Vec::new();
            let mut total = 0.0f64;
            for f in &faces {
                let Some(corners) = surface.face(*f) else {
                    continue;
                };
                if !real_polygon(surface, corners) {
                    continue;
                }
                let normal = surface.face_normal(*f);
                crate::triangulate::face_triangles(surface, corners, |[a, b, c]| {
                    let place = |i: usize| surface.position(corners[i]).unwrap_or(Vec3::ZERO);
                    let points = [place(a), place(b), place(c)];
                    let area = f64::from(
                        (points[1] - points[0])
                            .cross(points[2] - points[0])
                            .length(),
                    ) / 2.0;
                    if area > 0.0 && area.is_finite() {
                        total += area;
                        triangles.push((points, normal));
                        totals.push(total);
                    }
                });
            }
            if triangles.is_empty() || !(total > 0.0 && total.is_finite()) {
                return Err(ScatterProblem::NoArea);
            }
            let mut random = Random::new(crate::names::mix(u64::from(options.seed), RANDOM_KEY));
            let wanted = options.count as usize;
            let tries = u64::from(options.count) * ScatterOptions::TRIES_PER_COPY;
            let mut tried = 0u64;
            while out.len() < wanted && tried < tries {
                let key = tried;
                tried += 1;
                let pick = f64::from(random.unit()) * total;
                let index = totals
                    .partition_point(|t| *t <= pick)
                    .min(triangles.len() - 1);
                let ([a, b, c], normal) = triangles[index];
                // An even spread over the triangle.
                let (r1, r2) = (random.unit().sqrt(), random.unit());
                let position = a * (1.0 - r1) + b * (r1 * (1.0 - r2)) + c * (r1 * r2);
                push(&mut out, position, normal, RANDOM_KEY, key);
            }
        }
    }
    Ok(out)
}

/// `surface` with copies of `shapes` scattered over it (only over the
/// polygons in `only`, when given), or why it can't be made. `most` is the
/// most polygons the result may have.
pub fn scatter(
    surface: &Mesh,
    only: Option<&BTreeSet<FaceId>>,
    shapes: &[ScatterShape<'_>],
    options: &ScatterOptions,
    most: usize,
) -> Result<Mesh, ScatterProblem> {
    let options = options.clamped();
    let spots = spots(surface, only, &options, shapes.len())?;
    let polygons = spots.iter().fold(surface.face_count(), |sum, spot| {
        sum.saturating_add(shapes[spot.shape].mesh.face_count())
    });
    if polygons > most {
        return Err(ScatterProblem::TooManyPolygons { polygons, most });
    }
    let mut out = surface.clone();
    for spot in &spots {
        let shape = &shapes[spot.shape];
        let matrix = spot.matrix(&options) * shape.matrix;
        if matrix.is_finite() && matrix.determinant().abs() >= 1e-12 {
            append(&mut out, shape.mesh, matrix);
        }
    }
    Ok(out)
}

/// How many polygons a scatter of shapes with `polygons` polygons each
/// would add, at most, on `surface` (for estimating its work without
/// placing anything).
pub fn polygons_added(surface: &Mesh, options: &ScatterOptions, polygons: &[usize]) -> usize {
    let options = options.clamped();
    let copies = match options.place_on {
        PlaceOn::Points => surface.vertex_count(),
        PlaceOn::Centres => surface.face_count(),
        PlaceOn::Random => options.count as usize,
    }
    .min(ScatterOptions::MAX_COPIES as usize);
    let largest = polygons.iter().copied().max().unwrap_or(0);
    copies.saturating_mul(largest)
}

#[cfg(test)]
mod tests;
