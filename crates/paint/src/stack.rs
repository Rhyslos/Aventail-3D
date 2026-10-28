//! Changing a texture set's layer stack ([decision 0181](../../../docs/decisions/0181-paint-layers.md)):
//! adding, removing, moving, grouping and merging layers, as the Paint
//! layers panel does.
//!
//! A layer is found by its **path**: its place in the set's layers
//! (bottom first), then in its group's, and so on. Each function changes
//! the set and returns where the layer it worked on ended up, or `None`
//! (with the set unchanged) when it can't be done.

use crate::image::Image;
use crate::set::{Content, Layer, MAX_DEPTH, MAX_LAYERS, TextureSet};

/// A layer's place: indices from the set's layers inwards, bottom first.
pub type LayerPath = Vec<usize>;

impl TextureSet {
    /// The layer at `path`.
    pub fn layer(&self, path: &[usize]) -> Option<&Layer> {
        let (first, rest) = path.split_first()?;
        let mut layer = self.layers.get(*first)?;
        for i in rest {
            layer = layer.children()?.get(*i)?;
        }
        Some(layer)
    }

    /// The layer at `path`, to change.
    pub fn layer_mut(&mut self, path: &[usize]) -> Option<&mut Layer> {
        let (first, rest) = path.split_first()?;
        let mut layer = self.layers.get_mut(*first)?;
        for i in rest {
            layer = layer.children_mut()?.get_mut(*i)?;
        }
        Some(layer)
    }

    /// The list holding the layer at `path` (the set's layers or a
    /// group's), and its index there.
    fn siblings_mut(&mut self, path: &[usize]) -> Option<(&mut Vec<Layer>, usize)> {
        let (last, parent) = path.split_last()?;
        let list = if parent.is_empty() {
            &mut self.layers
        } else {
            self.layer_mut(parent)?.children_mut()?
        };
        (*last < list.len()).then_some((list, *last))
    }

    /// Every layer's path, top first as the panel lists them, with how
    /// deep it is in groups (0 for the set's own layers). A closed group's
    /// layers are left out unless `all`.
    pub fn listed(&self, all: bool) -> Vec<(LayerPath, usize)> {
        fn walk(
            layers: &[Layer],
            prefix: &mut LayerPath,
            all: bool,
            out: &mut Vec<(LayerPath, usize)>,
        ) {
            for (i, layer) in layers.iter().enumerate().rev() {
                prefix.push(i);
                out.push((prefix.clone(), prefix.len() - 1));
                if let Some(children) = layer.children()
                    && (all || layer.expanded)
                {
                    walk(children, prefix, all, out);
                }
                prefix.pop();
            }
        }
        let mut out = Vec::new();
        walk(&self.layers, &mut Vec::new(), all, &mut out);
        out
    }

    /// How many layers it has, groups' layers included.
    pub fn layer_count(&self) -> usize {
        self.listed(true).len()
    }

    /// The topmost paint layer, the brush's layer when none is chosen.
    pub fn top_paint_layer(&self) -> Option<LayerPath> {
        self.listed(true)
            .into_iter()
            .map(|(path, _)| path)
            .find(|path| self.layer(path).is_some_and(|l| l.image().is_some()))
    }

    /// Puts `layer` just above the layer at `above` (in its group), or on
    /// top of the set when `above` is `None` or not a layer. Its name is
    /// made unique. Returns its path.
    pub fn insert_layer(&mut self, mut layer: Layer, above: Option<&[usize]>) -> Option<LayerPath> {
        if self.layer_count() + layer.count() > MAX_LAYERS {
            return None;
        }
        layer.name = self.free_name(&layer.name);
        let at = above.filter(|p| self.layer(p).is_some());
        if let Some(path) = at {
            if path.len() + layer.depth() > MAX_DEPTH {
                return None;
            }
            let (list, index) = self.siblings_mut(path)?;
            list.insert(index + 1, layer);
            let mut out = path.to_vec();
            *out.last_mut()? += 1;
            return Some(out);
        }
        self.layers.push(layer);
        Some(vec![self.layers.len() - 1])
    }

    /// A new paint layer above `above` (see [`insert_layer`](Self::insert_layer)).
    pub fn add_paint_layer(&mut self, name: &str, above: Option<&[usize]>) -> Option<LayerPath> {
        let image = Image::new(self.size)?;
        self.insert_layer(Layer::paint(name, image), above)
    }

    /// Takes the layer at `path` out. Returns it.
    pub fn remove_layer(&mut self, path: &[usize]) -> Option<Layer> {
        let (list, index) = self.siblings_mut(path)?;
        Some(list.remove(index))
    }

    /// A copy of the layer at `path` just above it, named anew.
    pub fn duplicate_layer(&mut self, path: &[usize]) -> Option<LayerPath> {
        let copy = self.layer(path)?.clone();
        self.insert_layer(copy, Some(path))
    }

    /// Moves the layer at `path` one place up (`up`) or down among its
    /// group's layers. Returns its new path.
    pub fn move_layer(&mut self, path: &[usize], up: bool) -> Option<LayerPath> {
        let (list, index) = self.siblings_mut(path)?;
        let to = if up {
            (index + 1 < list.len()).then_some(index + 1)?
        } else {
            index.checked_sub(1)?
        };
        list.swap(index, to);
        let mut out = path.to_vec();
        *out.last_mut()? = to;
        Some(out)
    }

    /// Puts the layer at `path` in a new group called `name`, in its
    /// place. Returns the group's path.
    pub fn group_layer(&mut self, path: &[usize], name: &str) -> Option<LayerPath> {
        if path.len() + self.layer(path)?.depth() + 1 > MAX_DEPTH
            || self.layer_count() + 1 > MAX_LAYERS
        {
            return None;
        }
        let name = self.free_name(name);
        let (list, index) = self.siblings_mut(path)?;
        let layer = list.remove(index);
        list.insert(index, Layer::group(name, vec![layer]));
        Some(path.to_vec())
    }

    /// Takes the layers out of the group at `path`, in its place, and
    /// removes the group. Returns the path of the topmost of them.
    pub fn ungroup(&mut self, path: &[usize]) -> Option<LayerPath> {
        let (list, index) = self.siblings_mut(path)?;
        let Content::Group(children) = &list[index].content else {
            return None;
        };
        let children = children.clone();
        let count = children.len();
        list.splice(index..=index, children);
        let mut out = path.to_vec();
        *out.last_mut()? = (index + count).checked_sub(1)?;
        (count > 0).then_some(out)
    }

    /// Whether the layer at `path` can be merged into the one under it
    /// ([`Self::merge_down`]): there is one, it isn't a group, and they
    /// aren't both clipped. Quick: nothing is mixed (decision 0188).
    pub fn can_merge_down(&self, path: &[usize]) -> bool {
        let Some((last, parent)) = path.split_last() else {
            return false;
        };
        let list = if parent.is_empty() {
            &self.layers
        } else {
            match self.layer(parent).and_then(Layer::children) {
                Some(children) => children,
                None => return false,
            }
        };
        let (Some(upper), Some(lower)) = (
            list.get(*last),
            last.checked_sub(1).and_then(|b| list.get(b)),
        ) else {
            return false;
        };
        !matches!(lower.content, Content::Group(_)) && !(upper.clipped && lower.clipped)
    }

    /// Mixes the layer at `path` into the one under it (in its group),
    /// as the surface shows them, into one paint layer with the lower
    /// one's name. Strokes kept with either are dropped: the merged
    /// pixels can't be painted again stroke by stroke. Returns its path.
    pub fn merge_down(&mut self, path: &[usize]) -> Option<LayerPath> {
        if !self.can_merge_down(path) {
            return None;
        }
        let size = self.size;
        let (list, index) = self.siblings_mut(path)?;
        let below = index.checked_sub(1)?;
        let upper = &list[index];
        let lower = &list[below];
        // The lower layer mixed on its own, as Normal: what it shows.
        let mut base = lower.clone();
        base.blend = crate::Blend::Normal;
        base.clipped = false;
        base.visible = true;
        let mut top = upper.clone();
        top.clipped = upper.clipped;
        let pair = [base, top];
        let image = crate::composite::flatten(&pair, size)?;
        let mut merged = Layer::paint(lower.name.clone(), image);
        // Each realism channel either changes, mixed the same way
        // (decision 0207).
        for channel in crate::Channel::ALL {
            if pair.iter().all(|l| l.channels.get(channel).is_none()) {
                continue;
            }
            let both = TextureSet {
                size,
                layers: pair.to_vec(),
                realism: true,
            };
            let view = crate::channel::channel_view(&both, channel);
            let image = crate::composite::flatten(&view.layers, size)?;
            *merged.channels.get_mut(channel) = Some(crate::ChannelPart::Paint(image));
        }
        merged.blend = lower.blend;
        merged.clipped = lower.clipped;
        merged.visible = lower.visible;
        list.remove(index);
        list[below] = merged;
        let mut out = path.to_vec();
        *out.last_mut()? = below;
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Blend;
    use crate::composite::Composite;

    fn filled(size: u32, rgba: [u8; 4]) -> Image {
        let pixels: Vec<u8> = (0..size * size).flat_map(|_| rgba).collect();
        Image::from_rgba(size, size, size, &pixels).unwrap()
    }

    fn names(set: &TextureSet) -> Vec<(String, usize)> {
        set.listed(true)
            .into_iter()
            .map(|(p, d)| (set.layer(&p).unwrap().name.clone(), d))
            .collect()
    }

    fn sample() -> TextureSet {
        let mut set = TextureSet::new(64).unwrap();
        set.add_paint_layer("Base", None).unwrap();
        set.add_paint_layer("Detail", None).unwrap();
        set
    }

    #[test]
    fn layers_are_listed_top_first_and_go_above_the_chosen_one() {
        let mut set = sample();
        assert_eq!(names(&set), [("Detail".into(), 0), ("Base".into(), 0)]);
        let path = set
            .insert_layer(Layer::fill("Fill", [1, 2, 3, 255]), Some(&[0]))
            .unwrap();
        assert_eq!(path, [1]);
        assert_eq!(set.layers[1].name, "Fill");
        // Names are kept apart.
        let copy = set.duplicate_layer(&[1]).unwrap();
        assert_eq!(set.layer(&copy).unwrap().name, "Fill 2");
        assert_eq!(set.top_paint_layer(), Some(vec![3]));
    }

    #[test]
    fn layers_move_within_their_group_and_stop_at_the_ends() {
        let mut set = sample();
        assert_eq!(set.move_layer(&[0], true), Some(vec![1]));
        assert_eq!(set.layers[1].name, "Base");
        assert_eq!(set.move_layer(&[1], true), None, "already on top");
        assert_eq!(set.move_layer(&[0], false), None, "already at the bottom");
        assert_eq!(set.move_layer(&[7], true), None);
    }

    #[test]
    fn grouping_and_ungrouping_keep_the_layers_in_order() {
        let mut set = sample();
        let group = set.group_layer(&[1], "Group").unwrap();
        assert_eq!(group, [1]);
        let inside = set.insert_layer(Layer::fill("Tint", [9, 9, 9, 255]), Some(&[1, 0]));
        assert_eq!(inside, Some(vec![1, 1]));
        assert_eq!(
            names(&set),
            [
                ("Group".into(), 0),
                ("Tint".into(), 1),
                ("Detail".into(), 1),
                ("Base".into(), 0)
            ]
        );
        // A closed group's layers aren't listed.
        set.layer_mut(&[1]).unwrap().expanded = false;
        assert_eq!(set.listed(false).len(), 2);
        assert_eq!(set.ungroup(&[1]), Some(vec![2]));
        assert_eq!(
            names(&set),
            [("Tint".into(), 0), ("Detail".into(), 0), ("Base".into(), 0)]
        );
        assert_eq!(set.ungroup(&[0]), None, "not a group");
        assert!(set.is_valid());
    }

    #[test]
    fn groups_cannot_nest_past_the_limit() {
        let mut set = sample();
        let mut path = vec![0];
        for _ in 0..MAX_DEPTH + 2 {
            match set.group_layer(&path, "G") {
                Some(p) => path = [p, vec![0]].concat(),
                None => break,
            }
        }
        assert!(set.is_valid());
        assert!(path.len() <= MAX_DEPTH);
    }

    #[test]
    fn merge_down_keeps_what_was_shown() {
        let mut set = TextureSet::new(64).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(64, [200, 200, 200, 255])));
        let mut shade = Layer::paint("Shade", filled(64, [128, 128, 128, 255]));
        shade.blend = Blend::Multiply;
        set.layers.push(shade);
        let before = Composite::of(&set).unwrap().image().pixel(3, 3);
        assert_eq!(set.merge_down(&[1]), Some(vec![0]));
        assert_eq!(set.layers.len(), 1);
        assert_eq!(set.layers[0].name, "Base");
        assert!(set.layers[0].strokes.is_empty());
        let after = Composite::of(&set).unwrap().image().pixel(3, 3);
        assert_eq!(before, after);
        assert_eq!(set.merge_down(&[0]), None, "nothing under it");
    }

    #[test]
    fn can_merge_down_says_what_merge_down_does() {
        let mut set = sample();
        let group = set.group_layer(&[0], "Group").unwrap();
        set.layers
            .push(Layer::paint("Top", Image::new(64).unwrap()));
        let mut clipped = Layer::paint("Clipped", Image::new(64).unwrap());
        clipped.clipped = true;
        set.layers.push(clipped.clone());
        set.layers.push(clipped);
        let mut paths: Vec<LayerPath> = set.listed(true).into_iter().map(|(p, _)| p).collect();
        paths.extend([vec![], vec![9], vec![0, 7], vec![1, 0], group]);
        for path in paths {
            let quick = set.can_merge_down(&path);
            let done = set.clone().merge_down(&path).is_some();
            assert_eq!(quick, done, "{path:?}");
        }
    }

    #[test]
    fn removing_and_odd_paths_do_nothing_wrong() {
        let mut set = sample();
        assert!(set.layer(&[]).is_none());
        assert!(set.layer(&[0, 0]).is_none(), "not a group");
        assert!(set.remove_layer(&[5]).is_none());
        assert_eq!(set.remove_layer(&[0]).unwrap().name, "Base");
        assert_eq!(names(&set), [("Detail".into(), 0)]);
    }
}
