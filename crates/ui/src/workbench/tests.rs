//! Tests for the workbench, run through the real interface without a window.

use super::*;
use crate::viewport::NoRenderer;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn info() -> AppInfo {
    AppInfo {
        name: "Test App".into(),
        version: "9.9.9".into(),
        family: "Test Family".into(),
    }
}

/// A stand-in renderer: draws nothing (like having no GPU), and answers
/// every question about the point under the mouse at once, with `surface`.
#[derive(Default)]
struct TestRenderer {
    /// What is under the mouse, wherever it is; `None` for background.
    surface: Option<glam::Vec3>,
    /// Whether to answer questions at all (a renderer without picking
    /// answers nothing).
    picks: bool,
    /// How many questions were asked.
    asks: usize,
}

impl ViewportRenderer for TestRenderer {
    fn render(&mut self, view: &crate::ViewportView) -> Result<egui::TextureId, String> {
        NoRenderer::default().render(view)
    }

    fn pick(&mut self, ask: Option<crate::PickAsk>) -> crate::PickReply {
        let Some(ask) = ask.filter(|_| self.picks) else {
            return crate::PickReply::default();
        };
        self.asks += 1;
        crate::PickReply {
            answer: Some(crate::Picked {
                pixel: ask.pixel,
                point: self.surface,
            }),
            waiting: false,
        }
    }
}

/// The workbench plus every action it has returned so far.
struct State {
    workbench: Workbench,
    actions: Vec<WorkbenchAction>,
    renderer: TestRenderer,
}

fn harness_for(workbench: Workbench) -> Harness<'static, State> {
    harness_with(workbench, TestRenderer::default())
}

fn harness_with(workbench: Workbench, renderer: TestRenderer) -> Harness<'static, State> {
    Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(
            |ui, state: &mut State| {
                let actions = state.workbench.show(ui, &mut state.renderer);
                state.actions.extend(actions);
            },
            State {
                workbench,
                actions: Vec::new(),
                renderer,
            },
        )
}

/// A menu on the main menu bar (the panels and the viewport have menus
/// with some of the same names, lower down).
fn main_menu<'h>(harness: &'h Harness<'static, State>, name: &'h str) -> egui_kittest::Node<'h> {
    harness
        .get_all_by_label(name)
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap_or_else(|| panic!("no menu {name}"))
}

fn fresh() -> Harness<'static, State> {
    let mut harness = harness_for(Workbench::new(info(), None));
    harness.run();
    harness
}

/// The panels that show a tab: all but the viewport, whose own menu line
/// takes the tab's place.
fn tabbed_panels() -> impl Iterator<Item = Panel> {
    Panel::ALL.into_iter().filter(|p| *p != Panel::Viewport)
}

#[test]
fn all_panel_tabs_are_visible() {
    let harness = fresh();
    for panel in tabbed_panels() {
        assert!(
            harness.query_by_label(panel.title()).is_some(),
            "missing tab: {}",
            panel.title()
        );
    }
    assert!(harness.query_by_label(crate::VIEWPORT_LABEL).is_some());
}

#[test]
fn menus_follow_the_design_order() {
    let harness = fresh();
    let menus = [
        "File", "Edit", "Create", "Modes", "Select", "Tools", "Mesh", "UV", "View", "Window",
        "Help",
    ];
    let mut last_x = f32::MIN;
    for menu in menus {
        let node = main_menu(&harness, menu);
        let x = node.rect().min.x;
        assert!(x > last_x, "{menu} is out of order");
        last_x = x;
    }
}

#[test]
fn attributes_sit_above_objects_on_the_right() {
    let harness = fresh();
    let attributes = harness.get_by_label("Attributes").rect();
    let objects = harness.get_by_label("Objects").rect();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    assert!(
        attributes.min.y < objects.min.y,
        "Attributes must be on top"
    );
    assert!(
        attributes.min.x > viewport.min.x,
        "the managers are right of the viewport"
    );
    assert!((attributes.min.x - objects.min.x).abs() < 2.0, "one column");
}

#[test]
fn coordinates_sit_below_the_viewport() {
    let harness = fresh();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let coordinates = harness.get_by_label("Coordinates").rect();
    assert!(coordinates.min.y > viewport.min.y);
}

#[test]
fn ctrl_q_asks_to_quit() {
    let mut harness = fresh();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Q);
    harness.run();
    assert_eq!(harness.state().actions, vec![WorkbenchAction::Quit]);
}

#[test]
fn file_quit_asks_to_quit() {
    let mut harness = fresh();
    main_menu(&harness, "File").click();
    harness.run();
    // The button's label includes its shortcut, such as "Quit Ctrl+Q".
    harness.get_by_label_contains("Quit").click();
    harness.run();
    assert_eq!(harness.state().actions, vec![WorkbenchAction::Quit]);
}

#[test]
fn clicking_a_mode_selects_it() {
    let mut harness = fresh();
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    harness.get_by_label("Polygons mode").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Polygons);
    assert!(harness.query_by_label_contains("Polygons mode:").is_some());
}

#[test]
fn the_modes_menu_selects_a_mode_too() {
    let mut harness = fresh();
    main_menu(&harness, "Modes").click();
    harness.run();
    harness.get_by_label("Edges").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Edges);
}

#[test]
fn commands_that_are_not_built_yet_cannot_be_clicked() {
    let harness = fresh();
    for label in ["Cube", "Move", "Undo", "Snapping", "Lock the X axis"] {
        assert!(
            harness.get_by_label(label).accesskit_node().is_disabled(),
            "{label} should be disabled until it is built"
        );
    }
}

#[test]
fn reset_layout_restores_the_standard_arrangement() {
    // A user's own arrangement: every panel as a tab in one group.
    let mut harness = harness_for(Workbench::new(
        info(),
        Some(DockState::new(vec![
            Panel::Objects,
            Panel::Viewport,
            Panel::Attributes,
            Panel::Coordinates,
        ])),
    ));
    harness.run();
    main_menu(&harness, "View").click();
    harness.run();
    harness.get_by_label("Reset panel layout").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.layout().iter_all_tabs().count(), 4);
    assert!(workbench.note().is_some_and(|n| n.contains("reset")));
    let attributes = harness.get_by_label("Attributes").rect();
    let objects = harness.get_by_label("Objects").rect();
    assert!(attributes.min.y < objects.min.y);
}

#[test]
fn a_broken_saved_layout_is_replaced_and_explained() {
    let broken = DockState::new(vec![Panel::Viewport]);
    let mut harness = harness_for(Workbench::new(info(), Some(broken)));
    harness.run();
    assert!(harness.query_by_label("Attributes").is_some());
    assert!(harness.query_by_label_contains("standard layout").is_some());
}

#[test]
fn help_opens_the_about_box() {
    let mut harness = fresh();
    main_menu(&harness, "Help").click();
    harness.run();
    harness.get_by_label("About Test App").click();
    harness.run();
    assert!(harness.state().workbench.is_about_open());
    assert!(harness.query_by_label("Version 9.9.9").is_some());
    assert!(harness.query_by_label_contains("Test Family").is_some());
}

#[test]
fn window_title_shows_name_and_version() {
    let workbench = Workbench::new(info(), None);
    assert_eq!(workbench.window_title(), "Test App 9.9.9");
}

#[test]
fn panels_explain_what_is_coming() {
    let harness = fresh();
    assert!(harness.query_by_label("Nothing selected").is_some());
    assert!(harness.query_by_label("No objects yet").is_some());
}

#[test]
fn the_themed_interface_still_shows_everything() {
    let mut harness = harness_for(Workbench::new(info(), None));
    crate::theme::apply(&harness.ctx);
    harness.run();
    for panel in tabbed_panels() {
        assert!(harness.query_by_label(panel.title()).is_some());
    }
    harness.get_by_label("Points mode").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Points);
}

#[test]
fn coordinates_fit_their_rows_with_no_empty_space_below() {
    // With the real theme, whose rows are taller than egui's defaults.
    let mut harness = harness_for(Workbench::new(info(), None));
    crate::theme::apply(&harness.ctx);
    harness.run();
    harness.run();
    let tab = harness.get_by_label("Coordinates").rect();
    let last_row = harness.get_by_label("Rotation B").rect();
    // The window is 900 high and the status bar 24: the Coordinates panel
    // ends at about 876, a few pixels below its last row.
    assert!(
        876.0 - last_row.max.y < 24.0,
        "{} px of empty space under the last row",
        876.0 - last_row.max.y
    );
    assert!(tab.min.y > 700.0, "Coordinates start at {}", tab.min.y);
}

#[test]
fn a_saved_layout_keeps_the_users_sizes() {
    let saved = crate::layout::default_layout();
    let workbench = Workbench::new(info(), Some(saved));
    assert!(!workbench.fit_pending);
}

#[test]
fn coordinates_show_every_field_as_in_the_mockup() {
    let harness = fresh();
    for group in ["Position", "Size"] {
        for axis in ["X", "Y", "Z"] {
            assert!(harness.query_by_label(&format!("{group} {axis}")).is_some());
        }
    }
    for axis in ["H", "P", "B"] {
        assert!(
            harness
                .query_by_label(&format!("Rotation {axis}"))
                .is_some()
        );
    }
    // Fields fill most of their group's width, as in the mockup.
    assert!(harness.get_by_label("Position X").rect().width() > 150.0);
    assert!(harness.query_by_label_contains("Object (Rel)").is_some());
    assert!(harness.query_by_label("Apply").is_some());
}

/// A renderer that records what it was asked to draw.
#[derive(Default)]
struct Recorder {
    views: Vec<crate::viewport::ViewportView>,
}

impl ViewportRenderer for Recorder {
    fn render(&mut self, view: &crate::viewport::ViewportView) -> Result<egui::TextureId, String> {
        self.views.push(view.clone());
        Ok(egui::TextureId::User(7))
    }
}

#[test]
fn the_viewport_asks_for_an_image_the_size_of_its_panel() {
    let mut recorder = Recorder::default();
    let mut workbench = Workbench::new(info(), None);
    let mut harness = Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui(|ui| {
            workbench.show(ui, &mut recorder);
        });
    harness.run();
    let panel = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    drop(harness);
    let view = recorder.views.last().expect("the viewport rendered");
    assert_eq!(view.size_px[0], panel.width().round() as u32);
    assert_eq!(view.size_px[1], panel.height().round() as u32);
    assert_eq!(view.camera_position, workbench.viewport().camera().position);
}

#[test]
fn without_a_gpu_the_viewport_explains_why() {
    let harness = fresh();
    assert!(
        harness
            .query_by_label_contains("can't be shown: no graphics device")
            .is_some()
    );
    assert!(harness.query_by_label(crate::VIEWPORT_LABEL).is_some());
}

// ---------------------------------------------------------- navigation ---

/// Drags across the viewport's middle with `button`, holding `modifiers`,
/// in a few small steps as a real mouse would.
fn drag_viewport(
    harness: &mut Harness<'static, State>,
    button: egui::PointerButton,
    modifiers: Modifiers,
    by: egui::Vec2,
) {
    let start = viewport_spot(harness);
    harness.event(egui::Event::ModifiersChanged(modifiers));
    harness.event(egui::Event::PointerMoved(start));
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos: start,
        button,
        pressed: true,
        modifiers,
    });
    harness.step();
    let steps = 6;
    for i in 1..=steps {
        let at = start + by * (i as f32 / steps as f32);
        harness.event(egui::Event::PointerMoved(at));
        harness.step();
    }
    harness.event(egui::Event::PointerButton {
        pos: start + by,
        button,
        pressed: false,
        modifiers,
    });
    harness.step();
    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// A point in the viewport clear of the message shown in the middle when
/// there is no GPU (as in these tests).
fn viewport_spot(harness: &Harness<'static, State>) -> egui::Pos2 {
    harness.get_by_label(crate::VIEWPORT_LABEL).rect().center() - egui::vec2(0.0, 120.0)
}

fn camera(harness: &Harness<'static, State>) -> tools::Camera {
    *harness.state().workbench.viewport().camera()
}

#[test]
fn alt_left_drag_orbits_the_view() {
    let mut harness = fresh();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Primary,
        Modifiers::ALT,
        egui::vec2(120.0, 0.0),
    );
    let after = camera(&harness);
    assert!(
        after.yaw < before.yaw,
        "dragging right turns the scene right"
    );
    let navigator = harness.state().workbench.viewport().navigator();
    assert_eq!(navigator.pivot(), tools::camera::DEFAULT_TARGET);
    assert!(harness.state().workbench.viewport().active_drag().is_none());
}

#[test]
fn a_plain_left_drag_leaves_the_camera_alone() {
    let mut harness = fresh();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Primary,
        Modifiers::NONE,
        egui::vec2(120.0, 60.0),
    );
    assert_eq!(camera(&harness), before);
}

#[test]
fn alt_middle_drag_pans_the_view() {
    let mut harness = fresh();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Middle,
        Modifiers::ALT,
        egui::vec2(90.0, 40.0),
    );
    let after = camera(&harness);
    assert_eq!((after.yaw, after.pitch), (before.yaw, before.pitch));
    assert_ne!(after.position, before.position);
    let pivot = harness.state().workbench.viewport().navigator().pivot();
    assert_ne!(
        pivot,
        tools::camera::DEFAULT_TARGET,
        "the pivot moves along"
    );
}

#[test]
fn alt_right_drag_zooms_the_view() {
    let mut harness = fresh();
    let before = harness.state().workbench.viewport().navigator().distance();
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::ALT,
        egui::vec2(100.0, 0.0),
    );
    let after = harness.state().workbench.viewport().navigator().distance();
    assert!(
        after < before,
        "dragging right zooms in: {after} vs {before}"
    );
}

#[test]
fn the_mouse_wheel_zooms_over_the_viewport_only() {
    let mut harness = fresh();
    let before = harness.state().workbench.viewport().navigator().distance();
    let wheel = |harness: &mut Harness<'static, State>, at: egui::Pos2| {
        harness.event(egui::Event::PointerMoved(at));
        harness.step();
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    };
    let objects = harness.get_by_label("Objects").rect().center() + egui::vec2(0.0, 60.0);
    wheel(&mut harness, objects);
    let unchanged = harness.state().workbench.viewport().navigator().distance();
    assert_eq!(
        unchanged, before,
        "scrolling elsewhere leaves the view alone"
    );
    let spot = viewport_spot(&harness);
    wheel(&mut harness, spot);
    let after = harness.state().workbench.viewport().navigator().distance();
    assert!(after < before * 0.95, "one notch away from you zooms in");
}

#[test]
fn view_reset_view_returns_to_the_opening_camera() {
    let mut harness = fresh();
    drag_viewport(
        &mut harness,
        egui::PointerButton::Primary,
        Modifiers::ALT,
        egui::vec2(150.0, 80.0),
    );
    assert_ne!(camera(&harness), tools::Camera::default());
    main_menu(&harness, "View").click();
    harness.run();
    harness.get_by_label("Reset view").click();
    harness.run();
    let navigator = *harness.state().workbench.viewport().navigator();
    assert_eq!(navigator, tools::Navigator::default());
}

#[test]
fn the_status_bar_explains_navigation() {
    let harness = fresh();
    let hint = crate::navigation_hint(tools::Scheme::MouseOnly);
    assert!(harness.query_by_label(&hint).is_some());
    assert!(hint.contains("Right drag"));
}

// --------------------------------------------------------- grid detail ---

#[test]
fn the_grid_starts_calm_and_the_palette_toggles_fine_lines() {
    let mut harness = fresh();
    assert!(!harness.state().workbench.view_options().fine_grid);
    harness
        .get_by_label(crate::palette::GRID_DETAIL_LABEL)
        .click();
    harness.run();
    assert!(harness.state().workbench.view_options().fine_grid);
    harness
        .get_by_label(crate::palette::GRID_DETAIL_LABEL)
        .click();
    harness.run();
    assert!(!harness.state().workbench.view_options().fine_grid);
}

#[test]
fn the_grid_toggle_sits_on_the_left_palette() {
    let harness = fresh();
    let toggle = harness
        .get_by_label(crate::palette::GRID_DETAIL_LABEL)
        .rect();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let polygons = harness.get_by_label("Polygons mode").rect();
    assert!(toggle.max.x <= viewport.min.x, "left of the viewport");
    assert!(toggle.min.y > polygons.min.y, "below the modes");
    assert!(
        !harness
            .get_by_label(crate::palette::GRID_DETAIL_LABEL)
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn the_view_menu_toggles_fine_grid_lines_too() {
    let mut harness = fresh();
    main_menu(&harness, "View").click();
    harness.run();
    // The palette button has the same name; the menu's is a checkbox.
    harness
        .get_all_by_label(crate::palette::GRID_DETAIL_LABEL)
        .find(|node| node.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("a checkbox in the View menu")
        .click();
    harness.run();
    assert!(harness.state().workbench.view_options().fine_grid);
}

#[test]
fn remembered_view_options_are_restored() {
    let options = crate::ViewOptions {
        fine_grid: true,
        ..crate::ViewOptions::default()
    };
    let workbench = Workbench::new(info(), None).with_view_options(options);
    assert_eq!(workbench.view_options(), options);
    let mut harness = harness_for(workbench);
    harness.run();
    assert!(harness.state().workbench.view_options().fine_grid);
}

// ------------------------------------------------- navigation controls ---

fn open_picker(harness: &mut Harness<'static, State>) {
    harness.get_by_label(crate::PICKER_LABEL).click();
    harness.run();
}

#[test]
fn the_picker_lists_every_scheme_and_switches_between_them() {
    let mut harness = fresh();
    open_picker(&mut harness);
    for scheme in tools::Scheme::ALL {
        assert!(
            harness.query_by_label(scheme.name()).is_some(),
            "{scheme:?}"
        );
    }
    assert_eq!(
        harness.state().workbench.view_options().navigation.scheme,
        tools::Scheme::MouseOnly,
        "the standard"
    );
    harness.get_by_label("Middle orbits").click();
    harness.run();
    let settings = harness.state().workbench.view_options().navigation;
    assert_eq!(settings.scheme, tools::Scheme::MiddleOrbits);
    let hint = crate::navigation_hint(tools::Scheme::MiddleOrbits);
    assert!(
        harness.query_by_label(&hint).is_some(),
        "status bar follows"
    );
}

#[test]
fn the_picker_sits_in_the_bottom_right_of_the_viewport() {
    let harness = fresh();
    let picker = harness.get_by_label(crate::PICKER_LABEL).rect();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    assert!(viewport.contains_rect(picker));
    assert!(picker.max.x > viewport.max.x - 30.0);
    assert!(picker.max.y > viewport.max.y - 30.0);
}

#[test]
fn the_pickers_switches_change_the_settings() {
    let mut harness = fresh();
    open_picker(&mut harness);
    harness.get_by_label("Invert orbit direction").click();
    harness.run();
    harness
        .get_by_label("Alt + mouse works in every scheme")
        .click();
    harness.run();
    let settings = harness.state().workbench.view_options().navigation;
    assert!(settings.invert_orbit);
    assert!(!settings.alt_everywhere);
}

#[test]
fn options_menu_opens_the_picker() {
    let mut harness = fresh();
    harness.get_by_label("Options").click();
    harness.run();
    harness
        .get_by_label(&format!("{}…", crate::PICKER_LABEL))
        .click();
    harness.run();
    assert!(harness.query_by_label("Middle orbits").is_some());
}

#[test]
fn a_plain_right_drag_orbits_as_standard() {
    // Mouse only is the standard scheme: no keys needed.
    let mut harness = fresh();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::NONE,
        egui::vec2(120.0, 0.0),
    );
    assert!(camera(&harness).yaw < before.yaw);
}

#[test]
fn inverted_orbit_turns_the_other_way() {
    let mut options = crate::ViewOptions::default();
    options.navigation.invert_orbit = true;
    let mut harness = harness_for(Workbench::new(info(), None).with_view_options(options));
    harness.run();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Primary,
        Modifiers::ALT,
        egui::vec2(120.0, 0.0),
    );
    assert!(camera(&harness).yaw > before.yaw);
}

// ----------------------------------------------- the viewport's header ---

#[test]
fn the_viewport_has_its_own_menu_line_instead_of_a_tab() {
    let harness = fresh();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    for menu in ["Cameras", "Display", "Options", "Panel"] {
        let rect = harness.get_by_label(menu).rect();
        assert!(
            rect.max.y <= viewport.min.y + 1.0,
            "{menu} sits above the image"
        );
        assert!(rect.min.x >= viewport.min.x - 1.0);
    }
    assert!(harness.query_by_label("Viewport").is_none(), "no tab");
}

#[test]
fn dragging_the_header_buttons_moves_the_view() {
    use crate::viewport_chrome as chrome;
    for (label, check) in [
        (chrome::ORBIT_BUTTON, 0),
        (chrome::PAN_BUTTON, 1),
        (chrome::ZOOM_BUTTON, 2),
    ] {
        let mut harness = fresh();
        let before = *harness.state().workbench.viewport().navigator();
        let at = harness.get_by_label(label).rect().center();
        harness.event(egui::Event::PointerMoved(at));
        harness.step();
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        for i in 1..=6 {
            harness.event(egui::Event::PointerMoved(
                at + egui::vec2(12.0 * i as f32, 0.0),
            ));
            harness.step();
        }
        harness.event(egui::Event::PointerButton {
            pos: at + egui::vec2(72.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        let after = *harness.state().workbench.viewport().navigator();
        match check {
            0 => assert_ne!(after.camera().yaw, before.camera().yaw, "{label}"),
            1 => assert_ne!(after.pivot(), before.pivot(), "{label}"),
            _ => assert!(after.distance() < before.distance(), "{label}"),
        }
    }
}

#[test]
fn display_menu_toggles_the_fine_grid() {
    let mut harness = fresh();
    harness.get_by_label("Display").click();
    harness.run();
    harness
        .get_all_by_label(crate::palette::GRID_DETAIL_LABEL)
        .find(|node| node.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("a checkbox in the Display menu")
        .click();
    harness.run();
    assert!(harness.state().workbench.view_options().fine_grid);
}

#[test]
fn the_managers_have_their_own_menu_lines() {
    let harness = fresh();
    for menu in ["Mode", "Object"] {
        assert!(harness.query_by_label(menu).is_some(), "{menu}");
    }
}

#[test]
fn the_menu_bar_shows_the_layouts() {
    let harness = fresh();
    for name in ["Standard", "Modelling", "UV Edit"] {
        assert!(harness.query_by_label(name).is_some(), "{name}");
    }
    let help = harness.get_by_label("Help").rect();
    assert!(harness.get_by_label("Standard").rect().min.x > help.max.x);
}

// ------------------------------------------------------------- accent ---

#[test]
fn red_is_the_standard_accent_and_the_window_menu_changes_it() {
    let mut harness = fresh();
    assert_eq!(
        harness.state().workbench.accent(),
        crate::theme::Accent::Red
    );
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Red
    );
    main_menu(&harness, "Window").click();
    harness.run();
    harness.get_by_label_contains("Accent colour").click();
    harness.run();
    harness.get_by_label("Petrol blue").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.accent(),
        crate::theme::Accent::PetrolBlue
    );
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::PetrolBlue
    );
}

#[test]
fn a_remembered_accent_is_restored() {
    let workbench = Workbench::new(info(), None).with_accent(crate::theme::Accent::Olive);
    let mut harness = harness_for(workbench);
    harness.run();
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Olive
    );
}

#[test]
fn accent_swatches_stay_inside_their_menu() {
    let mut harness = fresh();
    main_menu(&harness, "Window").click();
    harness.run();
    harness.get_by_label_contains("Accent colour").click();
    harness.run();
    // The submenu opens to the right of "Accent colour". Each swatch is
    // drawn before its name, so the names leave room for one inside the
    // submenu, and nothing is drawn after them.
    let parent_right = harness.get_by_label_contains("Accent colour").rect().max.x;
    let lefts: Vec<f32> = crate::theme::Accent::ALL
        .iter()
        .map(|a| harness.get_by_label(a.name()).rect().min.x)
        .collect();
    for left in &lefts {
        assert!((left - lefts[0]).abs() < 0.5, "names line up");
        assert!(
            left - parent_right >= 16.0 + 6.0,
            "room for the swatch: {}",
            left - parent_right
        );
    }
}

#[test]
fn the_alt_mouse_scheme_can_still_be_picked() {
    let mut harness = fresh();
    open_picker(&mut harness);
    harness.get_by_label("Alt + mouse").click();
    harness.run();
    let before = camera(&harness);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::NONE,
        egui::vec2(120.0, 0.0),
    );
    assert_eq!(
        camera(&harness),
        before,
        "a plain right drag no longer orbits"
    );
}

// --------------------------------------- orbit and zoom around the mouse ---

/// A point on the test cube's front face, standing in for whatever is
/// under the mouse.
const SURFACE: glam::Vec3 = glam::Vec3::new(30.0, -100.0, 120.0);

fn with_surface(surface: Option<glam::Vec3>) -> Harness<'static, State> {
    let renderer = TestRenderer {
        surface,
        picks: true,
        asks: 0,
    };
    let mut harness = harness_with(Workbench::new(info(), None), renderer);
    harness.run();
    harness
}

fn pivot(harness: &Harness<'static, State>) -> glam::Vec3 {
    harness.state().workbench.viewport().navigator().pivot()
}

#[test]
fn orbiting_turns_around_the_surface_under_the_mouse() {
    let mut harness = with_surface(Some(SURFACE));
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::NONE,
        egui::vec2(100.0, 30.0),
    );
    assert_eq!(pivot(&harness), SURFACE);
}

#[test]
fn orbiting_over_the_background_keeps_the_pivot() {
    let mut harness = with_surface(None);
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::NONE,
        egui::vec2(100.0, 30.0),
    );
    assert_eq!(pivot(&harness), tools::camera::DEFAULT_TARGET);
}

#[test]
fn switching_off_around_the_mouse_keeps_the_old_pivot() {
    let mut harness = with_surface(Some(SURFACE));
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .navigation
        .around_cursor = false;
    drag_viewport(
        &mut harness,
        egui::PointerButton::Secondary,
        Modifiers::NONE,
        egui::vec2(100.0, 30.0),
    );
    assert_eq!(pivot(&harness), tools::camera::DEFAULT_TARGET);
}

#[test]
fn the_wheel_zooms_towards_the_surface_under_the_mouse() {
    let mut harness = with_surface(Some(SURFACE));
    let spot = viewport_spot(&harness);
    harness.event(egui::Event::PointerMoved(spot));
    harness.run();
    let before = harness
        .state()
        .workbench
        .viewport()
        .camera()
        .position
        .distance(SURFACE);
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(pivot(&harness), SURFACE);
    assert!(harness.state().workbench.viewport().navigator().distance() < before);
}

#[test]
fn the_wheel_over_the_ground_zooms_towards_the_ground() {
    // No surface anywhere: the ground under the mouse is the target.
    let mut harness = with_surface(None);
    let below_middle =
        harness.get_by_label(crate::VIEWPORT_LABEL).rect().center() + egui::vec2(-150.0, 180.0);
    harness.event(egui::Event::PointerMoved(below_middle));
    harness.run();
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let target = pivot(&harness);
    assert!(target.z.abs() < 0.5, "on the ground: {target}");
    // ...under the mouse, which zooming keeps it under.
    let body = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(body.width(), body.height());
    let seen = harness
        .state()
        .workbench
        .viewport()
        .camera()
        .project(target, size)
        .unwrap();
    let mouse = below_middle - body.min;
    assert!(
        (seen.x - mouse.x).abs() < 2.0 && (seen.y - mouse.y).abs() < 2.0,
        "{seen} vs {mouse:?}"
    );
}

#[test]
fn a_resting_mouse_asks_only_once() {
    let mut harness = with_surface(Some(SURFACE));
    let spot = viewport_spot(&harness);
    harness.event(egui::Event::PointerMoved(spot));
    harness.run();
    let asks = harness.state().renderer.asks;
    for _ in 0..5 {
        harness.step();
    }
    assert_eq!(harness.state().renderer.asks, asks, "no new questions");
    assert!(asks >= 1);
    harness.event(egui::Event::PointerMoved(spot + egui::vec2(20.0, 0.0)));
    harness.run();
    assert!(harness.state().renderer.asks > asks, "moving asks again");
}

#[test]
fn the_around_the_mouse_switch_is_in_the_picker_and_options() {
    let mut harness = fresh();
    open_picker(&mut harness);
    harness
        .get_by_label(crate::viewport::AROUND_CURSOR_LABEL)
        .click();
    harness.run();
    assert!(
        !harness
            .state()
            .workbench
            .view_options()
            .navigation
            .around_cursor
    );
}
