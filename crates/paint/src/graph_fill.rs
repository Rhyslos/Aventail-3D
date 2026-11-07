//! Graph fills (Phase 12, milestone 5,
//! [decision 0261](../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! a layer whose pixels are made by one of the project's material graphs,
//! worked out on the model it is painted on. Like a material layer
//! (decision 0210) it isn't painted into: the `bake` crate makes its
//! colour (and its roughness, metal and height with the realism channels)
//! from the graph, the model's baked maps and the layer's own values of
//! the settings the graph shows in the painter.

use matgraph::{BoxId, GraphId, SettingValue};

/// The most settings of its own a graph fill keeps.
pub const MAX_VALUES: usize = 256;

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
}

impl GraphFill {
    /// A graph fill running graph `graph` with its own settings, not made
    /// yet.
    pub fn of(graph: GraphId) -> Self {
        Self {
            graph,
            values: Vec::new(),
            made: 0,
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

    /// Whether its values are within the limits and real.
    pub fn is_valid(&self) -> bool {
        self.values.len() <= MAX_VALUES
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
}
