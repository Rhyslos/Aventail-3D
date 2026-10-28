//! The brush bar ([decision 0185](../../../docs/decisions/0185-brush-bar-and-tips.md)):
//! along the bottom of the Paint layout, in Coordinates' place (mockup A),
//! a row of brush cards (each with its sample stroke and name), filters,
//! a search box, and + to keep the brush as it is set now.
//!
//! A brush is a preset: a tip and the settings that make it (hardness,
//! spacing, flow and the pen curves). The size and colour are left as
//! they are, so changing brushes doesn't lose them.

use crate::attributes::BrushSettings;
use crate::text::tr;
use crate::theme;
use egui::{Color32, Rect, Sense, Ui, vec2};
use paint::BrushKind;
use paint::tip::Tip;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Accessible name of the bar's row of brushes, and the label tests look
/// for.
pub(crate) const BRUSH_ROW_LABEL: &str = crate::text!("Brush cards");

/// The + button's accessible name.
pub(crate) const KEEP_LABEL: &str = crate::text!("Keep this brush");

/// A card's size.
const CARD: egui::Vec2 = vec2(92.0, 58.0);

/// The most brushes a person may keep.
pub(crate) const MAX_SAVED: usize = 200;

/// A brush: a tip and the settings that make it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrushPreset {
    /// Its name.
    pub name: String,
    /// Its tip, by number (`paint::tip::Tip::index`).
    pub tip: u8,
    /// Hardness, 0 to 1.
    pub hardness: f32,
    /// Spacing, as a share of its width.
    pub spacing: f32,
    /// Flow, 0 to 1.
    pub flow: f32,
    /// Whether pressure changes the size, and its least share.
    pub size_pressure: Option<f32>,
    /// Whether pressure changes the flow, and its least share.
    pub flow_pressure: Option<f32>,
    /// What it does, by number (`paint::BrushKind::index`): paint, or an
    /// effect brush (decision 0186). Brushes kept before effects paint.
    #[serde(default)]
    pub kind: u8,
}

impl BrushPreset {
    fn new(name: &str, tip: Tip, hardness: f32, spacing: f32, flow: f32) -> Self {
        Self {
            name: name.to_string(),
            tip: tip.index(),
            hardness,
            spacing,
            flow,
            size_pressure: Some(0.1),
            flow_pressure: None,
            kind: 0,
        }
    }

    fn effect(name: &str, kind: BrushKind, hardness: f32, flow: f32) -> Self {
        Self {
            kind: kind.index(),
            size_pressure: None,
            ..Self::new(name, Tip::Round, hardness, 0.08, flow)
        }
    }

    /// The brush the settings make now, called `name`.
    pub(crate) fn of(name: &str, settings: &BrushSettings) -> Self {
        Self {
            name: name.chars().take(80).collect(),
            tip: settings.brush.tip.index(),
            hardness: settings.brush.hardness,
            spacing: settings.brush.spacing,
            flow: settings.brush.flow,
            size_pressure: settings
                .size_pressure
                .on
                .then_some(settings.size_pressure.least),
            flow_pressure: settings
                .flow_pressure
                .on
                .then_some(settings.flow_pressure.least),
            kind: match settings.brush.kind {
                // The eraser keeps a brush as a paint brush.
                BrushKind::Erase => 0,
                kind => kind.index(),
            },
        }
    }

    /// What it does.
    pub(crate) fn kind(&self) -> BrushKind {
        BrushKind::from_index(self.kind).unwrap_or_default()
    }

    /// Its tip.
    pub(crate) fn tip(&self) -> Tip {
        Tip::from_index(self.tip).unwrap_or_default()
    }

    /// Takes it up in `settings`, keeping their size and colour.
    pub(crate) fn apply(&self, settings: &mut BrushSettings) {
        settings.brush.tip = self.tip();
        settings.brush.hardness = self.hardness;
        settings.brush.spacing = self.spacing;
        settings.brush.flow = self.flow;
        settings.size_pressure.on = self.size_pressure.is_some();
        if let Some(least) = self.size_pressure {
            settings.size_pressure.least = least;
        }
        settings.flow_pressure.on = self.flow_pressure.is_some();
        if let Some(least) = self.flow_pressure {
            settings.flow_pressure.least = least;
        }
        let kind = self.kind();
        if kind != BrushKind::Paint || settings.brush.kind != BrushKind::Erase {
            settings.brush.kind = kind;
        }
        settings.tool = crate::attributes::PaintTool::Brush;
        settings.brush = settings.brush.clamped();
    }

    /// Whether it is what the settings are now.
    fn matches(&self, settings: &BrushSettings) -> bool {
        let now = Self::of(&self.name, settings);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let kind = |k: BrushKind| if k == BrushKind::Erase { 0 } else { k.index() };
        kind(settings.brush.kind) == self.kind
            && now.tip == self.tip
            && close(now.hardness, self.hardness)
            && close(now.spacing, self.spacing)
            && close(now.flow, self.flow)
    }

    /// Whether its numbers are in range.
    pub(crate) fn is_valid(&self) -> bool {
        let unit = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        self.name.chars().count() <= 80
            && Tip::from_index(self.tip).is_some()
            && unit(self.hardness)
            && unit(self.flow)
            && self.spacing.is_finite()
            && (0.01..=4.0).contains(&self.spacing)
            && self.size_pressure.is_none_or(unit)
            && self.flow_pressure.is_none_or(unit)
            && BrushKind::from_index(self.kind).is_some()
    }
}

/// The brushes that come with the app: the stylised set.
pub(crate) fn built_in() -> Vec<BrushPreset> {
    vec![
        BrushPreset::new(tr("Soft round"), Tip::Round, 0.0, 0.08, 0.6),
        BrushPreset::new(tr("Hard round"), Tip::Round, 0.95, 0.08, 1.0),
        BrushPreset::new(tr("Chalk"), Tip::Chalk, 0.6, 0.1, 0.9),
        BrushPreset::new(tr("Bristle"), Tip::Bristle, 0.7, 0.05, 0.8),
        BrushPreset::new(tr("Dry brush"), Tip::DryBrush, 0.8, 0.05, 0.9),
        BrushPreset {
            size_pressure: None,
            ..BrushPreset::new(tr("Spatter"), Tip::Spatter, 1.0, 0.6, 1.0)
        },
        BrushPreset::effect(tr("Smudge"), BrushKind::Smudge, 0.4, 0.8),
        BrushPreset::effect(tr("Blend"), BrushKind::Blend, 0.2, 0.5),
        BrushPreset::effect(tr("Blur"), BrushKind::Blur, 0.3, 0.6),
        BrushPreset::effect(tr("Hue shift"), BrushKind::HueShift, 0.5, 0.4),
        BrushPreset::effect(tr("Projection"), BrushKind::Projection, 0.6, 1.0),
        BrushPreset::effect(tr("Clone"), BrushKind::Clone, 0.6, 1.0),
    ]
}

/// Which brushes the bar shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Filter {
    #[default]
    All,
    /// The brushes that come with the app.
    BuiltIn,
    /// The brushes that change what is painted: smudge, blend, blur, hue
    /// shift, projection and clone.
    Effects,
    /// The person's own.
    Saved,
}

/// The brush bar's state.
#[derive(Default)]
pub(crate) struct BrushBar {
    filter: Filter,
    search: String,
    /// The name typed for a brush being kept, while + is open.
    naming: Option<String>,
    /// Each card's sample stroke, by tip and hardness.
    previews: HashMap<(u8, u32), egui::TextureHandle>,
}

impl std::fmt::Debug for BrushBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrushBar")
            .field("filter", &self.filter)
            .field("search", &self.search)
            .finish()
    }
}

/// What the bar changed in the person's own brushes.
pub(crate) enum SavedChange {
    Add(BrushPreset),
    Remove(usize),
    /// Save your own brushes in a preset file (decision 0206).
    SaveFile,
    /// Load brushes from a preset file.
    LoadFile,
}

impl BrushBar {
    /// Draws the bar; choosing a brush changes `settings`. Changes to the
    /// person's own brushes are returned.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        settings: &mut BrushSettings,
        saved: &[BrushPreset],
    ) -> Option<SavedChange> {
        let mut change = None;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.filter, Filter::All, tr("All"));
            ui.selectable_value(&mut self.filter, Filter::BuiltIn, tr("Built in"));
            ui.selectable_value(&mut self.filter, Filter::Effects, tr("Effects"));
            ui.selectable_value(&mut self.filter, Filter::Saved, tr("Saved"));
            let search = ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(tr("Search"))
                    .desired_width(110.0),
            );
            search.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr("Search brushes"))
            });
            match &mut self.naming {
                None => {
                    let keep = ui.button("+");
                    keep.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(KEEP_LABEL))
                    });
                    if keep
                        .on_hover_text(tr("Keep the brush as it is set now, under a name"))
                        .clicked()
                    {
                        self.naming = Some(tr("My brush").to_string());
                    }
                }
                Some(name) => {
                    let field = ui.add(egui::TextEdit::singleline(name).desired_width(110.0));
                    field.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::TextEdit,
                            true,
                            tr("Name of the brush to keep"),
                        )
                    });
                    let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ui.button(tr("Keep")).clicked() || enter) && !name.trim().is_empty() {
                        if saved.len() < MAX_SAVED {
                            change = Some(SavedChange::Add(BrushPreset::of(name.trim(), settings)));
                        }
                        self.naming = None;
                    } else if ui.button(tr("Cancel")).clicked() {
                        self.naming = None;
                    }
                }
            }
            if ui
                .add_enabled(!saved.is_empty(), egui::Button::new(tr("Save brushes…")))
                .on_hover_text(tr(
                    "Keep your own brushes in a preset file, to use in another project or on \
                     another computer, or to share",
                ))
                .clicked()
            {
                change = Some(SavedChange::SaveFile);
            }
            if ui
                .button(tr("Load brushes…"))
                .on_hover_text(tr("Add the brushes in a preset file to your own"))
                .clicked()
            {
                change = Some(SavedChange::LoadFile);
            }
        });
        let search = self.search.trim().to_lowercase();
        let shown = |p: &BrushPreset| search.is_empty() || p.name.to_lowercase().contains(&search);
        let built = built_in();
        let mut cards: Vec<(Option<usize>, &BrushPreset)> = Vec::new();
        let effect = |p: &BrushPreset| p.kind() != BrushKind::Paint;
        if self.filter != Filter::Saved {
            cards.extend(
                built
                    .iter()
                    .filter(|p| shown(p) && (self.filter != Filter::Effects || effect(p)))
                    .map(|p| (None, p)),
            );
        }
        if matches!(self.filter, Filter::All | Filter::Saved | Filter::Effects) {
            cards.extend(
                saved
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| shown(p) && (self.filter != Filter::Effects || effect(p)))
                    .map(|(i, p)| (Some(i), p)),
            );
        }
        let response = egui::ScrollArea::horizontal()
            .id_salt("brush_bar_cards")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if cards.is_empty() {
                        ui.label(
                            egui::RichText::new(tr("No brushes match.")).color(theme::TEXT_WEAK),
                        );
                    }
                    for (saved_at, preset) in &cards {
                        let chosen = preset.matches(settings);
                        let card = self.card(ui, preset, chosen);
                        if card.clicked() {
                            preset.apply(settings);
                        }
                        if let Some(i) = saved_at {
                            card.context_menu(|ui| {
                                if ui.button(tr("Delete this brush")).clicked() {
                                    change = Some(SavedChange::Remove(*i));
                                    ui.close();
                                }
                            });
                        }
                    }
                })
                .response
            })
            .inner;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(BRUSH_ROW_LABEL))
        });
        change
    }

    /// One brush's card: its sample stroke and name.
    fn card(&mut self, ui: &mut Ui, preset: &BrushPreset, chosen: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(CARD, Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                chosen,
                crate::trf!("{} {}", tr("Brush"), preset.name),
            )
        });
        let painter = ui.painter_at(rect);
        let accent = theme::accent(ui.ctx()).colors();
        let fill = if chosen {
            accent.fill.gamma_multiply(0.35)
        } else if response.hovered() {
            theme::TEXT_WEAK.gamma_multiply(0.15)
        } else {
            theme::TEXT_WEAK.gamma_multiply(0.06)
        };
        painter.rect_filled(rect, 5.0, fill);
        if chosen {
            painter.rect_stroke(
                rect,
                5.0,
                egui::Stroke::new(1.5, accent.light),
                egui::StrokeKind::Inside,
            );
        }
        let key = (preset.tip, (preset.hardness * 100.0).round() as u32);
        let texture = self.previews.entry(key).or_insert_with(|| {
            let (w, h) = (84u32, 32u32);
            let pixels = paint::tip::preview(preset.tip(), preset.hardness, w, h);
            ui.ctx().load_texture(
                format!("brush_preview_{}_{}", key.0, key.1),
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels),
                egui::TextureOptions::LINEAR,
            )
        });
        let image = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(84.0, 32.0));
        painter.image(
            texture.id(),
            image,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.text(
            egui::pos2(rect.center().x, rect.bottom() - 10.0),
            egui::Align2::CENTER_CENTER,
            &preset.name,
            egui::FontId::proportional(11.0),
            if chosen {
                theme::TEXT_STRONG
            } else {
                theme::TEXT
            },
        );
        response.on_hover_text(&preset.name)
    }
}
