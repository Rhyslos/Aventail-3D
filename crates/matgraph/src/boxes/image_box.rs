//! Image (Phase 12 milestone 21, [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! a photo or a painting brought in from a file, the start of a material
//! made from a picture.
//!
//! The box holds the picture ([`crate::Photo`], brought in through
//! Attributes' **Bring in a picture…**) and gives a square part of it,
//! chosen by **Square size**, **Left to right** and **Top to bottom**, at
//! the size the graph is worked out at, as a **Colour** and as its
//! lightness (**Lightness**, a height). A photo doesn't tile: Make
//! seamless after it does that, and Attributes' **Make into material**
//! adds the whole chain (decision 0283).
//!
//! With no picture brought in, the evaluator is told why
//! ([`BoxType::run_outputs`] refuses in plain words); [`BoxType::run`]
//! alone gives a plain grey, for callers wanting every output.

use super::{info, number};
use crate::image::Image;
use crate::kind::ValueKind;
use crate::painting::lightness;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::{text, tr};

/// The box's id.
pub const IMAGE: &str = "image";

/// The setting keys choosing the square part: its size and place.
pub const PART: &str = "part";
/// How far across the square sits (0 the left, 1 the right).
pub const ACROSS: &str = "across";
/// How far down the square sits (0 the top, 1 the bottom).
pub const DOWN: &str = "down";

/// What the box says when it holds no picture yet.
pub const NO_PICTURE: &str = text!(
    "No picture brought in yet. Choose Bring in a picture… in Attributes to pick a photo or a \
     painting (PNG, JPEG, TGA or BMP)."
);

/// The Image box.
#[derive(Debug)]
pub(crate) struct ImageBox {
    info: BoxInfo,
}

impl ImageBox {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                IMAGE,
                text!("Image"),
                Group::Make,
                text!(
                    "A photo or a painting brought in from a file (Bring in a picture… in \
                     Attributes). It gives a square part of it as a colour, and its lightness as \
                     a height. A photo doesn't repeat cleanly and has light and shadow baked in: \
                     Make into material (in Attributes) adds the boxes that fix that and guess \
                     its height and roughness, each with its own settings."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("grey", text!("Lightness"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    number(
                        PART,
                        text!("Square size"),
                        text!(
                            "How much of the picture the square part takes: 1 is the biggest \
                             square that fits (the picture's shorter side), 0.5 half that."
                        ),
                        crate::photo::MIN_PART,
                        1.0,
                        1.0,
                    ),
                    number(
                        ACROSS,
                        text!("Left to right"),
                        text!(
                            "Where the square sits across a wide picture: 0 at the left edge, \
                             1 at the right."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        DOWN,
                        text!("Top to bottom"),
                        text!(
                            "Where the square sits down a tall picture: 0 at the top, 1 at the \
                             bottom."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                ],
                &["even_light", "seamless", "guess_height", "guess_roughness"],
            ),
        }
    }
}

impl BoxType for ImageBox {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn takes_photo(&self) -> bool {
        true
    }

    /// The square part as a colour and its lightness; a plain grey with
    /// no picture.
    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let Some(photo) = job.photo() else {
            return Ok(vec![
                Value::Colour(Arc::new(Image::filled(size, [0.5; 3]))),
                Value::Height(Arc::new(Image::filled(size, 0.5))),
            ]);
        };
        let square = photo.square(job.number(PART), job.number(ACROSS), job.number(DOWN));
        let pixels = photo.sampled(square, size.get());
        let colour = Image::from_pixels(size, pixels)
            .ok_or_else(|| Refusal::new(tr("The picture couldn't be scaled.")))?;
        let grey = colour.map(lightness);
        Ok(vec![
            Value::Colour(Arc::new(colour)),
            Value::Height(Arc::new(grey)),
        ])
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        if job.photo().is_none() {
            return Err(Refusal::new(tr(NO_PICTURE)));
        }
        Ok(self.run(job)?.into_iter().map(Some).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BoxId, Graph, GraphEdit};
    use crate::image::Size;
    use crate::photo::tests::ramp;
    use crate::{Evaluator, Registry};
    use glam::Vec2;

    fn size32() -> Size {
        Size::new(32).unwrap()
    }

    fn graph_with(photo: Option<crate::Photo>) -> (Graph, BoxId) {
        let registry = Registry::standard();
        let mut g = Graph::new("Photo");
        let id = BoxId(g.next_id());
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: IMAGE.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        if let Some(p) = photo {
            g.apply(
                registry,
                &GraphEdit::SetPhoto {
                    node: id,
                    photo: Some(Arc::new(p)),
                },
            )
            .unwrap();
        }
        (g, id)
    }

    #[test]
    fn with_no_picture_the_box_says_how_to_bring_one_in() {
        let (g, id) = graph_with(None);
        let out = Evaluator::with_threads(2).run(Registry::standard(), &g, size32(), &[id]);
        let why = out.outcome(id).unwrap().as_ref().expect_err("refused");
        assert!(why.reason.contains("Bring in a picture"), "{}", why.reason);
    }

    #[test]
    fn the_square_part_comes_out_at_the_graph_size() {
        let (mut g, id) = graph_with(Some(ramp(200, 100)));
        let registry = Registry::standard();
        let mut e = Evaluator::with_threads(2);
        let out = e.run(registry, &g, size32(), &[id]);
        let result = out.outcome(id).unwrap().clone().unwrap();
        let colour = result.outputs[0].clone().unwrap();
        let c = colour.colour().unwrap();
        assert_eq!(c.size(), 32);
        // The middle square: red from about a quarter to three quarters.
        assert!((c.at(0, 16)[0] - 0.25).abs() < 0.05, "{:?}", c.at(0, 16));
        assert!((c.at(31, 16)[0] - 0.75).abs() < 0.05);
        // Lightness follows the colour.
        let grey = result.outputs[1].clone().unwrap();
        let expected = lightness(c.at(5, 5));
        assert!((grey.grey().unwrap().at(5, 5) - expected).abs() < 1e-5);
        // Moved to the right edge: only that box is worked out again.
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node: id,
                key: ACROSS.into(),
                value: crate::SettingValue::Number(1.0),
            },
        )
        .unwrap();
        let out = e.run(registry, &g, size32(), &[id]);
        let c = out.outcome(id).unwrap().clone().unwrap().outputs[0]
            .clone()
            .unwrap();
        assert!(c.colour().unwrap().at(31, 16)[0] > 0.95);
    }

    #[test]
    fn only_image_boxes_hold_a_picture_and_copies_keep_it() {
        let registry = Registry::standard();
        let (mut g, id) = graph_with(Some(ramp(4, 4)));
        let noise = BoxId(g.next_id());
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::new(300.0, 0.0),
            },
        )
        .unwrap();
        let refused = g.apply(
            registry,
            &GraphEdit::SetPhoto {
                node: noise,
                photo: Some(Arc::new(ramp(2, 2))),
            },
        );
        assert!(refused.is_err());
        let copies = g.get(id).unwrap().copy_edits(BoxId(99));
        assert!(
            copies
                .iter()
                .any(|e| matches!(e, GraphEdit::SetPhoto { photo: Some(_), .. }))
        );
        // Taken off again.
        g.apply(
            registry,
            &GraphEdit::SetPhoto {
                node: id,
                photo: None,
            },
        )
        .unwrap();
        assert!(g.get(id).unwrap().photo.is_none());
    }

    #[test]
    fn a_new_photo_remakes_the_box() {
        let registry = Registry::standard();
        let (mut g, id) = graph_with(Some(ramp(64, 64)));
        let mut e = Evaluator::with_threads(2);
        let first = e.run(registry, &g, size32(), &[id]);
        assert_eq!(first.made(), 1);
        let again = e.run(registry, &g, size32(), &[id]);
        assert_eq!(again.made(), 0);
        g.apply(
            registry,
            &GraphEdit::SetPhoto {
                node: id,
                photo: Some(Arc::new(ramp(64, 32))),
            },
        )
        .unwrap();
        assert_eq!(e.run(registry, &g, size32(), &[id]).made(), 1);
    }

    /// A photo that doesn't tile, lit from the left: the not-tiling
    /// test picture as bytes.
    fn lit_photo(n: u32) -> crate::Photo {
        let picture = crate::boxes::seamless::not_tiling(n);
        let rgb: Vec<u8> = picture
            .pixels()
            .iter()
            .flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
            .collect();
        crate::Photo::new(n, n, rgb, Vec::new(), "lit.jpg").unwrap()
    }

    #[test]
    fn a_photo_becomes_a_seamless_material_through_the_chain() {
        let registry = Registry::standard();
        let (mut g, image) = graph_with(Some(lit_photo(256)));
        let mut add = |g: &mut Graph, kind: &str, x: f32| {
            let id = BoxId(g.next_id());
            g.apply(
                registry,
                &GraphEdit::AddBox {
                    kind: kind.into(),
                    at: Vec2::new(x, 0.0),
                },
            )
            .unwrap();
            id
        };
        let even = add(&mut g, crate::boxes::even_light::EVEN_LIGHT, 300.0);
        let seamless = add(&mut g, crate::boxes::seamless::SEAMLESS, 600.0);
        let height = add(&mut g, crate::boxes::guess_height::GUESS_HEIGHT, 900.0);
        let rough = add(
            &mut g,
            crate::boxes::guess_roughness::GUESS_ROUGHNESS,
            900.0,
        );
        let made = add(&mut g, "material_from_height", 1200.0);
        let out = add(&mut g, "material_out", 1500.0);
        let wire = |g: &mut Graph, from: BoxId, fp: &str, to: BoxId, tp: &str| {
            g.apply(
                registry,
                &GraphEdit::Connect {
                    from: crate::PlugRef::new(from, fp),
                    to: crate::PlugRef::new(to, tp),
                },
            )
            .unwrap();
        };
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node: even,
                key: crate::boxes::even_light::EDGES.into(),
                value: crate::SettingValue::Choice(crate::boxes::even_light::PHOTO_EDGES),
            },
        )
        .unwrap();
        wire(&mut g, image, "colour", even, "colour");
        wire(&mut g, even, "colour_out", seamless, "colour");
        wire(&mut g, seamless, "colour_out", height, "colour");
        wire(&mut g, seamless, "colour_out", rough, "colour");
        wire(&mut g, height, "out", made, "height");
        wire(&mut g, seamless, "colour_out", made, "colour");
        wire(&mut g, rough, "out", made, "roughness");
        wire(&mut g, made, "material", out, "material");
        let mut e = Evaluator::with_threads(2);
        let material = e.material(registry, &g, Size::new(128).unwrap()).unwrap();
        let value = Value::Material(material.clone());
        let check = crate::SeamCheck::of(&value);
        assert!(!check.shows(), "{}", check.describe());
        crate::boxes::tests_each::assert_in_range("photo material", &[value]);
        // The roughness is the guessed one, within its standard range.
        let (lo, hi) = material
            .roughness
            .pixels()
            .iter()
            .fold((1.0f32, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
        assert!(
            lo >= 0.45 - 1e-4 && hi <= 0.9 + 1e-4 && hi - lo > 0.1,
            "{lo} {hi}"
        );
        // The light from the left is mostly gone from the colour.
        let c = &material.colour;
        let side = |x0: i64| -> f32 {
            (0..128)
                .flat_map(|y| (x0..x0 + 16).map(move |x| (x, y)))
                .map(|(x, y)| lightness(c.at(x, y)))
                .sum::<f32>()
                / (128.0 * 16.0)
        };
        let photo = Value::Colour(Arc::new(crate::boxes::seamless::not_tiling(128)));
        let before = {
            let p = photo.colour().unwrap();
            let s = |x0: i64| -> f32 {
                (0..128)
                    .flat_map(|y| (x0..x0 + 16).map(move |x| (x, y)))
                    .map(|(x, y)| lightness(p.at(x, y)))
                    .sum::<f32>()
                    / (128.0 * 16.0)
            };
            (s(24) - s(88)).abs()
        };
        let after = (side(24) - side(88)).abs();
        assert!(after < before * 0.5, "before {before} after {after}");
    }

    /// How long each photo box takes at 2048 pixels (run by hand:
    /// `cargo test -p matgraph --release -- --ignored photo_box_speed
    /// --nocapture`).
    #[test]
    #[ignore = "a timing, run by hand"]
    fn photo_box_speed() {
        use crate::registry::Job;
        let size = Size::new(2048).unwrap();
        let photo = Arc::new(lit_photo(2048));
        let registry = Registry::standard();
        let colour = Value::Colour(Arc::new(crate::boxes::seamless::not_tiling(2048)));
        for kind in [
            IMAGE,
            crate::boxes::even_light::EVEN_LIGHT,
            crate::boxes::seamless::SEAMLESS,
            crate::boxes::guess_height::GUESS_HEIGHT,
            crate::boxes::guess_roughness::GUESS_ROUGHNESS,
        ] {
            let t = registry.get(kind).unwrap();
            let inputs = t
                .info()
                .inputs
                .iter()
                .map(|p| (p.key == "colour").then(|| colour.clone()))
                .collect();
            let settings = crate::Settings::new();
            let job =
                Job::new(t.info(), size, 1, 2, &settings, inputs).with_photo(Some(photo.clone()));
            let start = std::time::Instant::now();
            t.run(&job).unwrap();
            println!("{kind}: {:?}", start.elapsed());
        }
    }
}
