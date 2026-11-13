//! Graph fills (Phase 12, milestone 5,
//! [decision 0261](../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! a layer whose pixels are made by one of the project's material graphs,
//! worked out on the model it is painted on. Like a material layer
//! (decision 0210) it isn't painted into: the `bake` crate makes its
//! colour (and its roughness, metal and height with the realism channels)
//! from the graph, the model's baked maps and the layer's own values of
//! the settings the graph shows in the painter.

use matgraph::{BoxId, GraphId, SettingValue, VariationId};

/// The most settings of its own a graph fill keeps.
pub const MAX_VALUES: usize = 256;

/// The smallest Tiling ([decision 0270](../../../docs/decisions/0270-tiling-and-moving-colours.md)):
/// the material eight times bigger than its real-world size.
pub const MIN_TILING: f32 = 0.125;

/// The largest Tiling: the material 32 times smaller.
pub const MAX_TILING: f32 = 32.0;

/// The longest setting key kept, in bytes.
const MAX_KEY: usize = 64;

/// One of a graph fill's own values: a setting of one box of the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphValue {
    /// The box.
    pub node: BoxId,
    /// The setting's key.
    pub key: String,
    /// The layer's value.
    pub value: SettingValue,
}

/// A graph fill's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphFill {
    /// The project's graph it runs.
    pub graph: GraphId,
    /// Its own values of the graph's shown settings: where the layer
    /// differs from the graph. A value whose box or setting is no longer
    /// in the graph is ignored.
    pub values: Vec<GraphValue>,
    /// What its pixels were last made from (the graph, these values, the
    /// model's shape and the texture size, hashed by whoever made them):
    /// 0 until they are made. When what it would be made from now hashes
    /// differently, it is made again.
    pub made: u64,
    /// How many times more often the material repeats on the model than
    /// at its real-world size (decision 0270): 2 makes it half the size,
    /// ½ twice. From [`MIN_TILING`] to [`MAX_TILING`]; 1 as made. Only
    /// the repeating parts change: what follows the model (the From the
    /// model boxes) stays where it is on the model.
    pub tiling: f32,
    /// The graph's variation it shows (milestone 18,
    /// [decision 0280](../../../docs/decisions/0280-variations-and-the-dice.md)):
    /// the graph made to look as that named look says before the layer's
    /// own values go on top. `None`, or a variation the graph no longer
    /// has: the graph as it is.
    pub variation: Option<VariationId>,
}

impl GraphFill {
    /// A graph fill running graph `graph` with its own settings, not made
    /// yet.
    pub fn of(graph: GraphId) -> Self {
        Self {
            graph,
            values: Vec::new(),
            made: 0,
            tiling: 1.0,
            variation: None,
        }
    }

    /// `tiling` brought within [`MIN_TILING`] and [`MAX_TILING`] (1 when
    /// it isn't a real number).
    pub fn usable_tiling(tiling: f32) -> f32 {
        if tiling.is_finite() {
            tiling.clamp(MIN_TILING, MAX_TILING)
        } else {
            1.0
        }
    }

    /// Its own value of setting `key` of box `node`, if it has one.
    pub fn value(&self, node: BoxId, key: &str) -> Option<SettingValue> {
        self.values
            .iter()
            .find(|v| v.node == node && v.key == key)
            .map(|v| v.value)
    }

    /// Sets its own value of setting `key` of box `node` (replacing any).
    pub fn set_value(&mut self, node: BoxId, key: &str, value: SettingValue) {
        match self
            .values
            .iter_mut()
            .find(|v| v.node == node && v.key == key)
        {
            Some(slot) => slot.value = value,
            None => self.values.push(GraphValue {
                node,
                key: key.to_string(),
                value,
            }),
        }
    }

    /// Forgets its own value of setting `key` of box `node`: back to the
    /// graph's.
    pub fn clear_value(&mut self, node: BoxId, key: &str) {
        self.values.retain(|v| !(v.node == node && v.key == key));
    }

    /// Whether its values and Tiling are within the limits and real.
    pub fn is_valid(&self) -> bool {
        (MIN_TILING..=MAX_TILING).contains(&self.tiling)
            && self.values.len() <= MAX_VALUES
            && self.values.iter().all(|v| {
                v.key.len() <= MAX_KEY
                    && match v.value {
                        SettingValue::Number(n) => n.is_finite(),
                        SettingValue::Colour(c) => c.iter().all(|x| x.is_finite()),
                        _ => true,
                    }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_values_are_set_replaced_and_cleared() {
        let mut fill = GraphFill::of(GraphId(3));
        assert!(fill.is_valid());
        assert_eq!(fill.value(BoxId(1), "amount"), None);
        fill.set_value(BoxId(1), "amount", SettingValue::Number(0.5));
        fill.set_value(BoxId(1), "amount", SettingValue::Number(0.7));
        fill.set_value(BoxId(2), "amount", SettingValue::Whole(3));
        assert_eq!(fill.values.len(), 2);
        assert_eq!(
            fill.value(BoxId(1), "amount"),
            Some(SettingValue::Number(0.7))
        );
        fill.clear_value(BoxId(1), "amount");
        assert_eq!(fill.value(BoxId(1), "amount"), None);
        assert_eq!(fill.values.len(), 1);
    }

    #[test]
    fn values_out_of_bounds_are_not_valid() {
        let mut fill = GraphFill::of(GraphId(1));
        fill.set_value(BoxId(1), "amount", SettingValue::Number(f32::NAN));
        assert!(!fill.is_valid());
        let mut fill = GraphFill::of(GraphId(1));
        fill.set_value(BoxId(1), &"k".repeat(65), SettingValue::Switch(true));
        assert!(!fill.is_valid());
        let mut fill = GraphFill::of(GraphId(1));
        for i in 0..=MAX_VALUES as u64 {
            fill.set_value(BoxId(i), "a", SettingValue::Whole(1));
        }
        assert!(!fill.is_valid());
    }

    #[test]
    fn tiling_starts_at_one_and_must_be_within_its_limits() {
        let mut fill = GraphFill::of(GraphId(1));
        assert_eq!(fill.tiling, 1.0);
        for good in [MIN_TILING, 0.5, 4.0, MAX_TILING] {
            fill.tiling = good;
            assert!(fill.is_valid(), "{good}");
        }
        for bad in [
            0.0,
            -1.0,
            MIN_TILING * 0.99,
            MAX_TILING * 1.01,
            f32::NAN,
            f32::INFINITY,
        ] {
            fill.tiling = bad;
            assert!(!fill.is_valid(), "{bad}");
        }
        assert_eq!(GraphFill::usable_tiling(100.0), MAX_TILING);
        assert_eq!(GraphFill::usable_tiling(0.0), MIN_TILING);
        assert_eq!(GraphFill::usable_tiling(f32::NAN), 1.0);
        assert_eq!(GraphFill::usable_tiling(3.0), 3.0);
    }
}
