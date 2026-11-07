//! Giving a graph's material to a shape without painting (Phase 12,
//! milestone 6, [decision 0262](../../../../docs/decisions/0262-baked-export.md)).
//!
//! A material made by a graph is a texture set holding one graph fill
//! layer (decision 0261): the workbench makes its pixels on the model, at
//! the material's real-world size, whenever the graph, the shape or its
//! UVs change. Everything else follows from that: the viewport shows it,
//! undo takes it back, the project saves it, the painter can paint over
//! it, and the texture export bakes it with every engine preset.
//!
//! This module holds the rules, with no interface: what a material's
//! texture set becomes when a graph is given to it, and whether a set is
//! "made by a graph".

use matgraph::GraphId;
use paint::{GraphFill, Image, Layer, TextureSet};

/// How a graph was given to a material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Given {
    /// The material had no textures: it is now made by the graph.
    New,
    /// It was made by another graph: now by this one.
    Replaced,
    /// It was painted: the graph goes under the paint, which stays.
    UnderPaint,
}

/// The graph a material's texture set is made by: a set holding one graph
/// fill and nothing else.
pub(crate) fn given_graph(set: &TextureSet) -> Option<GraphId> {
    match set.layers.as_slice() {
        [only] => only.graph.as_ref().map(|f| f.graph),
        _ => None,
    }
}

/// `set` (or a new one `size` pixels across) with graph `id`, called
/// `name`, given to it, and how; `None` when it can't take another layer.
/// A set made by a graph changes graph; a painted one gets the graph as
/// its bottom layer, under the paint. Its realism channels are turned on,
/// so the material's roughness, metal and height come with it.
pub(crate) fn with_graph_given(
    set: Option<&TextureSet>,
    id: GraphId,
    name: &str,
    size: u32,
) -> Option<(TextureSet, Given)> {
    let name: String = name.chars().take(paint::set::MAX_NAME - 8).collect();
    let name = if name.trim().is_empty() {
        crate::text::tr("Graph").to_string()
    } else {
        name
    };
    let mut set = match set {
        Some(set) => set.clone(),
        None => TextureSet::new(size)?,
    };
    let layer = |set: &TextureSet| -> Option<Layer> {
        let mut layer = Layer::paint(set.free_name(&name), Image::new(set.size)?);
        layer.graph = Some(GraphFill::of(id));
        Some(layer)
    };
    let given = if set.layers.is_empty() {
        let layer = layer(&set)?;
        set.layers.push(layer);
        Given::New
    } else if given_graph(&set).is_some() {
        set.layers.clear();
        let layer = layer(&set)?;
        set.layers.push(layer);
        Given::Replaced
    } else {
        if set.layers.len() >= paint::set::MAX_LAYERS {
            return None;
        }
        let layer = layer(&set)?;
        set.layers.insert(0, layer);
        Given::UnderPaint
    };
    set.realism = true;
    set.is_valid().then_some((set, given))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_material_becomes_made_by_the_graph() {
        let (set, given) = with_graph_given(None, GraphId(3), "Stone", 64).unwrap();
        assert_eq!(given, Given::New);
        assert_eq!(set.size, 64);
        assert!(set.realism, "roughness, metal and height come with it");
        assert_eq!(given_graph(&set), Some(GraphId(3)));
        assert_eq!(set.layers[0].name, "Stone");
        assert_eq!(
            set.layers[0].graph.as_ref().unwrap().made,
            0,
            "not made yet"
        );
    }

    #[test]
    fn another_graph_replaces_the_first() {
        let (set, _) = with_graph_given(None, GraphId(3), "Stone", 64).unwrap();
        let (set, given) = with_graph_given(Some(&set), GraphId(5), "Wood", 64).unwrap();
        assert_eq!(given, Given::Replaced);
        assert_eq!(set.layers.len(), 1);
        assert_eq!(given_graph(&set), Some(GraphId(5)));
        assert_eq!(set.layers[0].name, "Wood");
    }

    #[test]
    fn paint_stays_on_top_of_the_graph() {
        let mut painted = TextureSet::new(32).unwrap();
        painted
            .layers
            .push(Layer::paint("Scratches", Image::new(32).unwrap()));
        let (set, given) = with_graph_given(Some(&painted), GraphId(1), "Metal", 64).unwrap();
        assert_eq!(given, Given::UnderPaint);
        assert_eq!(set.size, 32, "the set keeps its size");
        assert_eq!(set.layers.len(), 2);
        assert!(set.layers[0].graph.is_some(), "at the bottom");
        assert_eq!(set.layers[1].name, "Scratches");
        // Painted and given: not "made by a graph" alone.
        assert_eq!(given_graph(&set), None);
    }

    #[test]
    fn names_are_kept_apart_and_never_empty() {
        let mut painted = TextureSet::new(32).unwrap();
        painted
            .layers
            .push(Layer::paint("Stone", Image::new(32).unwrap()));
        let (set, _) = with_graph_given(Some(&painted), GraphId(1), "Stone", 32).unwrap();
        assert_ne!(set.layers[0].name, set.layers[1].name);
        let (set, _) = with_graph_given(None, GraphId(1), "  ", 32).unwrap();
        assert_eq!(set.layers[0].name, "Graph");
        let long = "x".repeat(500);
        assert!(with_graph_given(None, GraphId(1), &long, 32).is_some());
    }

    #[test]
    fn a_full_set_or_a_bad_size_is_refused() {
        assert!(with_graph_given(None, GraphId(1), "Stone", 3).is_none());
        let mut full = TextureSet::new(16).unwrap();
        for i in 0..paint::set::MAX_LAYERS {
            full.layers
                .push(Layer::paint(format!("L{i}"), Image::new(16).unwrap()));
        }
        assert!(with_graph_given(Some(&full), GraphId(1), "Stone", 16).is_none());
    }
}
