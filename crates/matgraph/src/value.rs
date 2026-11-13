//! What travels along a wire: a picture of one [`ValueKind`].
//!
//! Pictures are shared ([`Arc`]): a box that passes one on, or a material
//! made of its inputs' pictures, copies nothing.

use crate::image::{Grey, Image, Rgb, Size};
use crate::kind::ValueKind;
use std::sync::Arc;

/// A whole material: every map, the same size.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialMaps {
    /// Its colour (sRGB, 0 to 1).
    pub colour: Arc<Rgb>,
    /// Its height, 0 (low) to 1 (high).
    pub height: Arc<Grey>,
    /// How rough it is: 0 shiny, 1 matt.
    pub roughness: Arc<Grey>,
    /// How metal it is: 0 not, 1 bare metal.
    pub metal: Arc<Grey>,
    /// Which way its surface faces, a direction per pixel (x right, y up
    /// the texture, z out of it: the OpenGL way, as Godot reads it).
    pub normal: Arc<Rgb>,
    /// How open it is to light: 1 open, darker in dips and corners.
    pub occlusion: Arc<Grey>,
    /// How it bends: 0.5 flat, lighter on ridges, darker in grooves.
    pub curvature: Arc<Grey>,
    /// How solid it is, for alpha cards (leaves, grass, fences): 1 solid,
    /// 0 see-through. `None`: solid everywhere, as every material was
    /// before the Opacity box (milestone 14, decision 0276).
    pub opacity: Option<Arc<Grey>>,
    /// How much light passes through it from behind (a leaf lit by the
    /// sun): 0 none, 1 most. `None`: none.
    pub translucency: Option<Arc<Grey>>,
}

impl MaterialMaps {
    /// Pixels across.
    pub fn size(&self) -> u32 {
        self.colour.size()
    }

    /// How much memory its maps take, in bytes (shared ones counted each
    /// time).
    pub fn memory_size(&self) -> usize {
        self.colour.memory_size()
            + self.height.memory_size()
            + self.roughness.memory_size()
            + self.metal.memory_size()
            + self.normal.memory_size()
            + self.occlusion.memory_size()
            + self.curvature.memory_size()
            + self.opacity.as_ref().map_or(0, |g| g.memory_size())
            + self.translucency.as_ref().map_or(0, |g| g.memory_size())
    }

    /// Its opacity at pixel (`x`, `y`) (wrapping): 1 where it has none.
    pub fn opacity_at(&self, x: i64, y: i64) -> f32 {
        self.opacity.as_ref().map_or(1.0, |g| g.at(x, y))
    }

    /// Whether it is see-through anywhere (it has an opacity map).
    pub fn is_cut_out(&self) -> bool {
        self.opacity.is_some()
    }
}

/// A picture of one kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A black-and-white shape.
    Shape(Arc<Grey>),
    /// A height.
    Height(Arc<Grey>),
    /// A colour picture.
    Colour(Arc<Rgb>),
    /// A whole material.
    Material(Arc<MaterialMaps>),
}

impl Value {
    /// Its kind.
    pub fn kind(&self) -> ValueKind {
        match self {
            Self::Shape(_) => ValueKind::Shape,
            Self::Height(_) => ValueKind::Height,
            Self::Colour(_) => ValueKind::Colour,
            Self::Material(_) => ValueKind::Material,
        }
    }

    /// Pixels across.
    pub fn size(&self) -> u32 {
        match self {
            Self::Shape(g) | Self::Height(g) => g.size(),
            Self::Colour(c) => c.size(),
            Self::Material(m) => m.size(),
        }
    }

    /// How much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        match self {
            Self::Shape(g) | Self::Height(g) => g.memory_size(),
            Self::Colour(c) => c.memory_size(),
            Self::Material(m) => m.memory_size(),
        }
    }

    /// Its greys, for a shape or a height.
    pub fn grey(&self) -> Option<&Arc<Grey>> {
        match self {
            Self::Shape(g) | Self::Height(g) => Some(g),
            _ => None,
        }
    }

    /// Its colours, for a colour picture.
    pub fn colour(&self) -> Option<&Arc<Rgb>> {
        match self {
            Self::Colour(c) => Some(c),
            _ => None,
        }
    }

    /// Its maps, for a whole material.
    pub fn material(&self) -> Option<&Arc<MaterialMaps>> {
        match self {
            Self::Material(m) => Some(m),
            _ => None,
        }
    }

    /// The same picture as `kind`, converted the way [`crate::kind::fit`]
    /// describes; `None` for kinds that don't convert.
    pub fn converted(&self, kind: ValueKind) -> Option<Value> {
        let greys_as_colour = |g: &Grey| -> Arc<Rgb> { Arc::new(g.map(|v| [v, v, v])) };
        Some(match (self, kind) {
            (v, k) if v.kind() == k => v.clone(),
            (Self::Shape(g), ValueKind::Height) => Self::Height(g.clone()),
            (Self::Height(g), ValueKind::Shape) => Self::Shape(g.clone()),
            (Self::Shape(g) | Self::Height(g), ValueKind::Colour) => {
                Self::Colour(greys_as_colour(g))
            }
            (Self::Material(m), ValueKind::Height) => Self::Height(m.height.clone()),
            (Self::Material(m), ValueKind::Colour) => Self::Colour(m.colour.clone()),
            _ => return None,
        })
    }

    /// A plain grey picture of `kind` (a shape or a height) at `size`.
    pub fn flat_grey(kind: ValueKind, size: Size, grey: f32) -> Option<Value> {
        let image = Arc::new(Image::filled(size, grey));
        match kind {
            ValueKind::Shape => Some(Self::Shape(image)),
            ValueKind::Height => Some(Self::Height(image)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::{Fit, fit};

    fn every_kind(size: Size) -> Vec<Value> {
        let grey = Arc::new(Image::filled(size, 0.25));
        let rgb = Arc::new(Image::filled(size, [0.1, 0.2, 0.3]));
        let maps = MaterialMaps {
            colour: rgb.clone(),
            height: grey.clone(),
            roughness: grey.clone(),
            metal: grey.clone(),
            normal: rgb.clone(),
            occlusion: grey.clone(),
            curvature: grey.clone(),
            opacity: None,
            translucency: None,
        };
        vec![
            Value::Shape(grey.clone()),
            Value::Height(grey),
            Value::Colour(rgb),
            Value::Material(Arc::new(maps)),
        ]
    }

    #[test]
    fn values_convert_exactly_where_kinds_fit() {
        let size = Size::new(16).unwrap();
        for value in every_kind(size) {
            for kind in ValueKind::ALL {
                let converted = value.converted(kind);
                match fit(value.kind(), kind) {
                    Ok(Fit::Fits | Fit::Converts(_)) => {
                        let c = converted.unwrap();
                        assert_eq!(c.kind(), kind);
                        assert_eq!(c.size(), 16);
                    }
                    Err(_) => assert!(converted.is_none()),
                }
            }
        }
    }

    #[test]
    fn a_grey_as_a_colour_is_the_grey_in_every_channel() {
        let size = Size::new(16).unwrap();
        let shape = &every_kind(size)[0];
        let colour = shape.converted(ValueKind::Colour).unwrap();
        assert_eq!(colour.colour().unwrap().at(3, 3), [0.25; 3]);
        assert_eq!(shape.memory_size(), 16 * 16 * 4);
    }
}
