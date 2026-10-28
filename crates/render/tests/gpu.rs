//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. CI sets it in the window test job, which has a software
//! Vulkan driver, so these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use glam::{Vec2, Vec3};
use render::{
    FrameParams, PickRequest, TEST_CUBE_SIZE, ViewportRenderer, ViewportTarget, best_sample_count,
    read_pixels,
};
use tools::Camera;

/// The viewport backdrop, sRGB 0 to 1 (as in the interface theme).
const SKY_TOP: [u8; 3] = [0x4a, 0x4a, 0x4a];
const SKY_HORIZON: [u8; 3] = [0x3e, 0x3e, 0x3e];
const GROUND: [u8; 3] = [0x38, 0x38, 0x38];

fn srgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| f32::from(v) / 255.0)
}

/// A focus point so far away that no grid is drawn near the camera.
const NO_GRID: Vec3 = Vec3::new(1.0e7, 1.0e7, 0.0);

struct Gpu {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
    let found =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()
            .and_then(|adapter| {
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                        .ok()?;
                Some(Gpu {
                    adapter,
                    device,
                    queue,
                })
            });
    if found.is_none() {
        assert!(
            std::env::var_os("REQUIRE_GPU").is_none(),
            "REQUIRE_GPU is set but no GPU adapter was found"
        );
        eprintln!("No GPU adapter available; skipping GPU rendering test.");
    }
    found
}

/// A rendered image and the camera that produced it.
struct Shot {
    pixels: Vec<u8>,
    size: [u32; 2],
    camera: Camera,
}

impl Shot {
    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size[0] + x) * 4) as usize;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }

    /// The pixel where a world point appears.
    fn at(&self, point: Vec3) -> [u8; 4] {
        let size = Vec2::new(self.size[0] as f32, self.size[1] as f32);
        let p = self.camera.project(point, size).expect("point is in view");
        self.pixel(p.x as u32, p.y as u32)
    }
}

fn brightness(p: [u8; 4]) -> i32 {
    i32::from(p[0]) + i32::from(p[1]) + i32::from(p[2])
}

/// Renders what `camera` sees, with the grid around `focus`.
fn render(
    gpu: &Gpu,
    renderer: &ViewportRenderer,
    target: &ViewportTarget,
    camera: Camera,
    focus: Vec3,
) -> Shot {
    render_with(gpu, renderer, target, camera, focus, None)
}

fn render_with(
    gpu: &Gpu,
    renderer: &ViewportRenderer,
    target: &ViewportTarget,
    camera: Camera,
    focus: Vec3,
    fine_spacing: Option<f32>,
) -> Shot {
    let size = target.size();
    let params = FrameParams {
        view_projection: camera.view_projection(Vec2::new(size[0] as f32, size[1] as f32)),
        camera_position: camera.position,
        focus,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        fine_spacing,
        flat_height: camera.ortho_height,
        grid_plane: render::GROUND_PLANE,
    };
    renderer.render(&gpu.device, &gpu.queue, target, &params);
    let pixels = read_pixels(&gpu.device, &gpu.queue, target).unwrap();
    // Set DUMP_DIR to a folder to save every rendered frame there as a PPM
    // image, for looking at what a failing test saw.
    if let Some(dir) = std::env::var_os("DUMP_DIR") {
        let mut ppm = format!("P6 {} {} 255\n", size[0], size[1]).into_bytes();
        for px in pixels.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::fs::write(std::path::Path::new(&dir).join(format!("{n}.ppm")), ppm).unwrap();
    }
    Shot {
        pixels,
        size,
        camera,
    }
}

fn setup(gpu: &Gpu, size: [u32; 2]) -> (ViewportRenderer, ViewportTarget) {
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let target = renderer.create_target(&gpu.device, size);
    (renderer, target)
}

fn close_to(got: [u8; 4], want: [u8; 3], tolerance: i32) -> bool {
    (0..3).all(|c| (i32::from(got[c]) - i32::from(want[c])).abs() <= tolerance)
}

#[test]
fn the_sky_is_above_the_horizon_and_the_ground_below() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [320, 200]);
    // Looking level, away from the test cube and the Z axis at the origin:
    // the horizon runs across the middle.
    let level = Camera::looking_at(
        Vec3::new(5000.0, -1000.0, 100.0),
        Vec3::new(5000.0, 0.0, 100.0),
    );
    let shot = render(&gpu, &renderer, &target, level, NO_GRID);
    let above = shot.pixel(160, 90);
    let below = shot.pixel(160, 110);
    assert!(close_to(above, SKY_HORIZON, 3), "{above:?}");
    assert!(close_to(below, GROUND, 3), "{below:?}");
    let top = shot.pixel(160, 1);
    assert!(
        brightness(top) > brightness(above),
        "the sky lightens upwards"
    );
    let bottom = shot.pixel(160, 198);
    assert!(
        brightness(bottom) < brightness(below),
        "the ground darkens downwards"
    );
    assert_eq!(top[3], 255);
}

#[test]
fn looking_up_shows_the_sky_and_down_shows_the_ground() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [320, 200]);
    let up = Camera::looking_at(
        Vec3::new(3000.0, 3000.0, 0.0),
        Vec3::new(3000.0, 3000.01, 100.0),
    );
    let shot = render(&gpu, &renderer, &target, up, NO_GRID);
    assert!(
        close_to(shot.pixel(160, 100), SKY_TOP, 3),
        "{:?}",
        shot.pixel(160, 100)
    );
    let down = Camera::looking_at(
        Vec3::new(3000.0, 3000.0, 500.0),
        Vec3::new(3000.0, 3000.01, 0.0),
    );
    let shot = render(&gpu, &renderer, &target, down, NO_GRID);
    let middle = shot.pixel(160, 100);
    assert!(brightness(middle) < i32::from(GROUND[0]) * 3, "{middle:?}");
    assert!(
        close_to(middle, [middle[0]; 3], 2),
        "neutral grey: {middle:?}"
    );
}

#[test]
fn the_cube_is_shaded_like_a_studio_light() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    let shot = render(
        &gpu,
        &renderer,
        &target,
        Camera::default(),
        tools::camera::DEFAULT_TARGET,
    );
    let h = TEST_CUBE_SIZE / 2.0;
    // Off-centre: the green Z axis comes out of the middle of the top.
    let top = shot.at(Vec3::new(40.0, -40.0, TEST_CUBE_SIZE));
    let front = shot.at(Vec3::new(0.0, -h, TEST_CUBE_SIZE / 2.0));
    let right = shot.at(Vec3::new(h, 0.0, TEST_CUBE_SIZE / 2.0));
    let sky = shot.pixel(10, 10);
    // Top lightest, then the front, then the right side, as in the mockup;
    // the top is lighter than the background.
    assert!(brightness(top) > brightness(front), "{top:?} vs {front:?}");
    assert!(
        brightness(front) > brightness(right),
        "{front:?} vs {right:?}"
    );
    assert!(brightness(top) > brightness(sky) + 60);
    // Neutral grey: no colour cast.
    for p in [top, front, right] {
        assert!((i32::from(p[0]) - i32::from(p[2])).abs() < 12, "{p:?}");
    }
}

#[test]
fn the_x_axis_is_red_and_the_y_axis_blue() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    // From above, so the axes are easy to hit and the cube is out of the way.
    let camera = Camera::looking_at(
        Vec3::new(-600.0, -600.0, 900.0),
        Vec3::new(-300.0, -300.0, 0.0),
    );
    let shot = render(
        &gpu,
        &renderer,
        &target,
        camera,
        Vec3::new(-300.0, -300.0, 0.0),
    );
    let size = Vec2::new(640.0, 400.0);
    let redness = |p: [u8; 4]| i32::from(p[0]) - i32::from(p[1]);
    let blueness = |p: [u8; 4]| i32::from(p[2]) - i32::from(p[0]).max(i32::from(p[1]));
    // The best match within a couple of pixels of where each axis passes.
    let best = |point: Vec3, score: &dyn Fn([u8; 4]) -> i32| {
        let c = camera.project(point, size).unwrap();
        let mut best = i32::MIN;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let p = shot.pixel((c.x as i32 + dx) as u32, (c.y as i32 + dy) as u32);
                best = best.max(score(p));
            }
        }
        best
    };
    assert!(
        best(Vec3::new(-400.0, 0.0, 0.0), &redness) > 25,
        "X axis not red"
    );
    assert!(
        best(Vec3::new(0.0, -400.0, 0.0), &blueness) > 15,
        "Y axis not blue"
    );
}

#[test]
fn grid_lines_are_lighter_than_the_ground_between_them() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    let camera = Camera::looking_at(
        Vec3::new(-300.0, -300.0, 600.0),
        Vec3::new(-300.0, -300.0, 0.0),
    );
    let shot = render(
        &gpu,
        &renderer,
        &target,
        camera,
        Vec3::new(-300.0, -300.0, 0.0),
    );
    // x = -400 is a 1 m line; x = -455, y = -255 is between all lines.
    let on_line = shot.at(Vec3::new(-400.0, -255.0, 0.0));
    let between = shot.at(Vec3::new(-455.0, -255.0, 0.0));
    assert!(
        brightness(on_line) > brightness(between) + 6,
        "{on_line:?} vs {between:?}"
    );
}

#[test]
fn fine_and_half_metre_grid_lines_show_only_when_chosen() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    let camera = Camera::looking_at(
        Vec3::new(-300.0, -300.0, 600.0),
        Vec3::new(-300.0, -300.0, 0.0),
    );
    // Looking almost straight down, world X runs up the screen and Y
    // across it, so these points share a screen row (and so the same
    // background shade): y = -210 is a 10 cm line only, y = -255 lies
    // between all lines, y = -200 is a 1 m line.
    let fine_line = Vec3::new(-455.0, -210.0, 0.0);
    let between = Vec3::new(-455.0, -255.0, 0.0);
    let on = render_with(
        &gpu,
        &renderer,
        &target,
        camera,
        Vec3::new(-300.0, -300.0, 0.0),
        Some(10.0),
    );
    assert!(
        brightness(on.at(fine_line)) > brightness(on.at(between)) + 3,
        "{:?} vs {:?}",
        on.at(fine_line),
        on.at(between)
    );
    let off = render_with(
        &gpu,
        &renderer,
        &target,
        camera,
        Vec3::new(-300.0, -300.0, 0.0),
        None,
    );
    assert!(
        (brightness(off.at(fine_line)) - brightness(off.at(between))).abs() <= 2,
        "{:?} vs {:?}",
        off.at(fine_line),
        off.at(between)
    );
    // The 1 m lines are there either way.
    let metre_line = Vec3::new(-455.0, -200.0, 0.0);
    assert!(brightness(off.at(metre_line)) > brightness(off.at(between)) + 6);
    // Half-metre lines: y = -250 is one, y = -210 and y = -225 are not.
    let half = render_with(
        &gpu,
        &renderer,
        &target,
        camera,
        Vec3::new(-300.0, -300.0, 0.0),
        Some(50.0),
    );
    let half_line = Vec3::new(-455.0, -250.0, 0.0);
    let off_line = Vec3::new(-455.0, -225.0, 0.0);
    assert!(
        brightness(half.at(half_line)) > brightness(half.at(off_line)) + 3,
        "{:?} vs {:?}",
        half.at(half_line),
        half.at(off_line)
    );
    assert!(
        (brightness(half.at(fine_line)) - brightness(half.at(off_line))).abs() <= 2,
        "no 10 cm lines in the half-metre grid"
    );
}

#[test]
fn resizing_makes_a_new_target_of_the_right_size() {
    let Some(gpu) = gpu() else { return };
    let (renderer, _) = setup(&gpu, [64, 64]);
    let target = renderer.create_target(&gpu.device, [300, 150]);
    assert_eq!(target.size(), [300, 150]);
    let zero = renderer.create_target(&gpu.device, [0, 0]);
    assert_eq!(zero.size(), [1, 1], "empty sizes become 1×1, never a crash");
    let shot = render(
        &gpu,
        &renderer,
        &target,
        Camera::default(),
        tools::camera::DEFAULT_TARGET,
    );
    assert_eq!(shot.pixels.len(), 300 * 150 * 4);
}

fn pick_request(camera: &Camera, pixel: Vec2, size: [u32; 2]) -> PickRequest {
    PickRequest {
        view_projection: camera.view_projection(Vec2::new(size[0] as f32, size[1] as f32)),
        pixel: pixel.to_array(),
        size,
    }
}

#[test]
fn picking_finds_the_surface_under_a_pixel() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, _) = setup(&gpu, [8, 8]);
    let size = [640, 400];
    let camera = Camera::default();
    let h = TEST_CUBE_SIZE / 2.0;
    for surface in [
        Vec3::new(0.0, -h, 100.0),
        Vec3::new(h, 30.0, 150.0),
        Vec3::new(-40.0, -40.0, TEST_CUBE_SIZE),
    ] {
        let pixel = camera
            .project(surface, Vec2::new(640.0, 400.0))
            .expect("in view");
        let result = renderer
            .pick_now(&gpu.device, &gpu.queue, pick_request(&camera, pixel, size))
            .expect("an answer");
        let point = result.point.expect("the cube is there");
        assert!((point - surface).length() < 2.0, "{point} vs {surface}");
        assert_eq!(result.pixel, pixel.to_array());
    }
}

#[test]
fn picking_the_background_finds_nothing() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, _) = setup(&gpu, [8, 8]);
    let camera = Camera::default();
    // The sky in the top-left corner; the grid and axes are not surfaces.
    for pixel in [Vec2::new(10.0, 10.0), Vec2::new(40.0, 390.0)] {
        let result = renderer
            .pick_now(
                &gpu.device,
                &gpu.queue,
                pick_request(&camera, pixel, [640, 400]),
            )
            .expect("an answer");
        assert_eq!(result.point, None, "{pixel}");
    }
}

#[test]
fn picking_never_waits_and_asks_one_question_at_a_time() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, _) = setup(&gpu, [8, 8]);
    let camera = Camera::default();
    let request = pick_request(&camera, Vec2::new(320.0, 200.0), [640, 400]);
    assert!(renderer.request_pick(&gpu.device, &gpu.queue, request));
    assert!(renderer.is_picking());
    assert!(
        !renderer.request_pick(&gpu.device, &gpu.queue, request),
        "busy: the second question is refused"
    );
    let mut answer = None;
    for _ in 0..200 {
        answer = renderer.poll_pick(&gpu.device);
        if answer.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(answer.is_some(), "the answer arrives on a later poll");
    assert!(!renderer.is_picking());
    assert!(
        renderer.request_pick(&gpu.device, &gpu.queue, request),
        "free again"
    );
}

#[test]
fn a_broken_pick_request_is_refused() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, _) = setup(&gpu, [8, 8]);
    let camera = Camera::default();
    let mut request = pick_request(&camera, Vec2::new(f32::NAN, 1.0), [640, 400]);
    assert!(!renderer.request_pick(&gpu.device, &gpu.queue, request));
    request.pixel = [1.0, 1.0];
    request.size = [0, 0];
    assert!(
        renderer
            .pick_now(&gpu.device, &gpu.queue, request)
            .is_none()
    );
}

/// Renders a flat front view (looking along +Y) of the test cube, with
/// the grid standing upright in the X–Z plane.
fn render_front(gpu: &Gpu, renderer: &ViewportRenderer, target: &ViewportTarget) -> Shot {
    let view = tools::Navigator::for_view(tools::ViewKind::Front);
    let camera = *view.camera();
    let size = target.size();
    let params = FrameParams {
        view_projection: camera.view_projection(Vec2::new(size[0] as f32, size[1] as f32)),
        camera_position: camera.position,
        focus: view.pivot(),
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        fine_spacing: None,
        flat_height: camera.ortho_height,
        grid_plane: [Vec3::X, Vec3::Z],
    };
    renderer.render(&gpu.device, &gpu.queue, target, &params);
    let pixels = read_pixels(&gpu.device, &gpu.queue, target).unwrap();
    Shot {
        pixels,
        size,
        camera,
    }
}

#[test]
fn a_flat_view_has_a_plain_backdrop_and_an_upright_grid() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    let shot = render_front(&gpu, &renderer, &target);
    // x = 300 is a 1 m line standing up beside the cube; x = 350 is between.
    let on_line = shot.at(Vec3::new(300.0, 0.0, 250.0));
    let between = shot.at(Vec3::new(350.0, 0.0, 250.0));
    assert!(
        brightness(on_line) > brightness(between) + 6,
        "{on_line:?} vs {between:?}"
    );
    // The same backdrop top and bottom: no horizon in a flat view.
    let (top, bottom) = (shot.pixel(5, 3), shot.pixel(5, 396));
    assert!(
        (brightness(top) - brightness(bottom)).abs() <= 3,
        "{top:?} vs {bottom:?}"
    );
    // The cube's front face is there, flat on, lit and neutral.
    let face = shot.at(Vec3::new(0.0, -100.0, 100.0));
    assert!(brightness(face) > brightness(between) + 150, "{face:?}");
}

#[test]
fn picking_works_in_a_flat_view() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, _) = setup(&gpu, [8, 8]);
    let view = tools::Navigator::for_view(tools::ViewKind::Top);
    let camera = *view.camera();
    let top_of_cube = Vec3::new(30.0, -20.0, TEST_CUBE_SIZE);
    let pixel = camera
        .project(top_of_cube, Vec2::new(640.0, 400.0))
        .unwrap();
    let result = renderer
        .pick_now(
            &gpu.device,
            &gpu.queue,
            pick_request(&camera, pixel, [640, 400]),
        )
        .unwrap();
    let point = result.point.expect("the cube's top");
    assert!((point - top_of_cube).length() < 2.0, "{point}");
}

#[test]
fn without_a_shape_only_the_backdrop_is_drawn_and_nothing_is_picked() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    let camera = Camera::default();
    let centre = Vec3::new(0.0, 0.0, TEST_CUBE_SIZE / 2.0);
    let middle = camera
        .project(centre, Vec2::new(320.0, 200.0))
        .expect("in view");
    let with_cube = render(&gpu, &renderer, &target, camera, Vec3::ZERO);
    renderer.set_shape(&gpu.device, &render::Shape::default());
    let without = render(&gpu, &renderer, &target, camera, Vec3::ZERO);
    assert!(
        brightness(with_cube.at(centre)) > brightness(without.at(centre)) + 60,
        "the light grey cube is gone"
    );
    let result = renderer
        .pick_now(
            &gpu.device,
            &gpu.queue,
            pick_request(&camera, middle, [320, 200]),
        )
        .expect("an answer");
    assert_eq!(result.point, None, "nothing to pick");
}

/// Whether a pixel is clearly the overlay's pure red.
fn is_red(p: [u8; 4]) -> bool {
    p[0] > 180 && p[1] < 80 && p[2] < 80
}

#[test]
fn the_overlay_draws_points_thick_lines_and_tints_hidden_behind_surfaces() {
    use render::Overlay;
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [640, 400]);
    let camera = Camera::default();
    let h = TEST_CUBE_SIZE / 2.0;
    let red = [1.0, 0.0, 0.0, 1.0];
    let near_corner = Vec3::new(h, -h, TEST_CUBE_SIZE);
    let far_corner = Vec3::new(-h, h, 0.0);
    let mut overlay = Overlay::default();
    overlay.point(near_corner, 10.0, red);
    overlay.point(far_corner, 10.0, red);
    renderer.set_overlay(&gpu.device, &overlay);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(
        is_red(shot.at(near_corner)),
        "the near corner's point shows"
    );
    // Points are round: red beside the middle, but not in the corners
    // of the square they are drawn in.
    let p = camera
        .project(near_corner, Vec2::new(640.0, 400.0))
        .unwrap();
    let (x, y) = (p.x as u32, p.y as u32);
    assert!(is_red(shot.pixel(x + 3, y)), "the dot is 10 pixels across");
    assert!(
        !is_red(shot.pixel(x + 4, y + 4)) && !is_red(shot.pixel(x - 4, y - 4)),
        "round: the square's corners are cut away"
    );
    assert!(
        !is_red(shot.at(far_corner)),
        "the far corner's is hidden by the cube"
    );

    // A thick line along the front-top edge: red a few pixels to the side.
    let a = Vec3::new(-h, -h, TEST_CUBE_SIZE);
    let b = Vec3::new(h, -h, TEST_CUBE_SIZE);
    let mut overlay = Overlay::default();
    overlay.line(a, b, 6.0, red);
    renderer.set_overlay(&gpu.device, &overlay);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    let size = Vec2::new(640.0, 400.0);
    let middle = camera.project(a.lerp(b, 0.5), size).unwrap();
    assert!(is_red(shot.pixel(middle.x as u32, middle.y as u32)));
    assert!(
        is_red(shot.pixel(middle.x as u32, middle.y as u32 + 2)),
        "thick"
    );
    assert!(
        !is_red(shot.pixel(middle.x as u32, middle.y as u32 + 12)),
        "not too thick"
    );

    // A see-through tint over the front face.
    let mut overlay = Overlay::default();
    overlay.triangle(
        [
            Vec3::new(-h, -h, 0.0),
            Vec3::new(h, -h, 0.0),
            Vec3::new(h, -h, TEST_CUBE_SIZE),
        ],
        [1.0, 0.0, 0.0, 0.5],
    );
    renderer.set_overlay(&gpu.device, &overlay);
    let before = shot.at(Vec3::new(h * 0.6, -h, TEST_CUBE_SIZE * 0.2));
    let tinted = render(&gpu, &renderer, &target, camera, NO_GRID).at(Vec3::new(
        h * 0.6,
        -h,
        TEST_CUBE_SIZE * 0.2,
    ));
    assert!(
        i32::from(tinted[0]) > i32::from(before[0]) + 20,
        "{before:?} -> {tinted:?}"
    );
    assert!(tinted[1] < before[1]);

    // An empty overlay draws nothing.
    renderer.set_overlay(&gpu.device, &Overlay::default());
    let clean = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(!is_red(clean.at(near_corner)));
}

#[test]
fn lines_on_top_show_through_surfaces() {
    // Decision 0158: a deformer's axis runs through its shape, so it is
    // drawn over everything, where an ordinary overlay line is hidden.
    use render::Overlay;
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [640, 400]);
    let camera = Camera::default();
    let h = TEST_CUBE_SIZE / 2.0;
    let red = [1.0, 0.0, 0.0, 1.0];
    // Up through the middle of the cube, hidden inside it.
    let (a, b) = (Vec3::ZERO, Vec3::Z * TEST_CUBE_SIZE);
    let middle = Vec3::new(0.0, 0.0, h);
    let mut overlay = Overlay::default();
    overlay.line(a, b, 6.0, red);
    renderer.set_overlay(&gpu.device, &overlay);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(!is_red(shot.at(middle)), "an ordinary line is hidden");
    let mut overlay = Overlay::default();
    overlay.line_on_top(a, b, 6.0, red);
    assert!(!overlay.is_empty());
    renderer.set_overlay(&gpu.device, &overlay);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(is_red(shot.at(middle)), "a line on top shows through");
}

// ---------------------------------------------------------- object meshes ---

/// The cube as an object mesh, the renderer's own test cube removed.
fn with_object_cube(gpu: &Gpu, renderer: &mut ViewportRenderer) -> render::MeshGeometry {
    renderer.set_shape(&gpu.device, &render::Shape::default());
    let cube = mesh::PrimitiveKind::Cube.standard().mesh();
    let geometry = render::MeshGeometry::new(&cube);
    renderer.set_mesh(&gpu.device, 7, &geometry);
    geometry
}

fn place(model: glam::Mat4) -> render::MeshDraw {
    render::MeshDraw {
        key: 7,
        model,
        edge_color: [1.0, 0.0, 0.0, 1.0],
        edges: true,
        faces: true,
        points: None,
        faintest: 0.0,
        palette: [0; render::PALETTE_SIZE],
        surface: render::SurfaceDraw::Plain,
    }
}

#[test]
fn object_meshes_are_drawn_and_picked_where_they_are_placed() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    let size = Vec2::new(320.0, 200.0);
    // Not placed yet: not drawn.
    let empty = render(&gpu, &renderer, &target, camera, NO_GRID);
    let middle = Vec3::new(0.0, 0.0, 50.0);
    renderer.set_draws(&gpu.device, &gpu.queue, &[place(glam::Mat4::IDENTITY)]);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(
        brightness(shot.at(middle)) > brightness(empty.at(middle)) + 60,
        "the grey cube shows"
    );
    let pixel = camera.project(middle, size).unwrap();
    let picked = renderer
        .pick_now(
            &gpu.device,
            &gpu.queue,
            pick_request(&camera, pixel, [320, 200]),
        )
        .and_then(|r| r.point)
        .expect("the cube is picked");
    assert!(picked.y < -45.0, "on its front face: {picked}");
    // Placed a metre to the right, it is drawn there instead.
    let moved = glam::Mat4::from_translation(Vec3::X * 150.0);
    renderer.set_draws(&gpu.device, &gpu.queue, &[place(moved)]);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(brightness(shot.at(middle)) < brightness(empty.at(middle)) + 20);
    assert!(brightness(shot.at(middle + Vec3::X * 150.0)) > brightness(empty.at(middle)) + 60);
    // Forgotten, it isn't drawn.
    renderer.keep_meshes(&[]);
    assert_eq!(renderer.mesh_count(), 0);
    let gone = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(brightness(gone.at(middle + Vec3::X * 150.0)) < brightness(empty.at(middle)) + 20);
}

#[test]
fn an_outline_only_mesh_shows_no_faces_and_is_not_picked() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    let size = Vec2::new(320.0, 200.0);
    let empty = render(&gpu, &renderer, &target, camera, NO_GRID);
    let outline = render::MeshDraw {
        faces: false,
        ..place(glam::Mat4::IDENTITY)
    };
    renderer.set_draws(&gpu.device, &gpu.queue, &[outline]);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    // The middle of the front face stays the background: no surface.
    let middle = Vec3::new(0.0, -50.0, 50.0);
    assert!(brightness(shot.at(middle)).abs_diff(brightness(empty.at(middle))) < 20);
    // Its edges still show (the red outline at a front corner's edge).
    let edge = Vec3::new(0.0, -50.0, 100.0);
    assert!(shot.at(edge) != empty.at(edge), "the outline is drawn");
    let pixel = camera.project(middle, size).unwrap();
    let picked = renderer
        .pick_now(
            &gpu.device,
            &gpu.queue,
            pick_request(&camera, pixel, [320, 200]),
        )
        .and_then(|r| r.point);
    assert!(picked.is_none(), "nothing to pick: {picked:?}");
}

#[test]
fn moved_points_are_updated_in_place_and_mirrors_shade_the_right_way() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    let mut geometry = with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    renderer.set_draws(&gpu.device, &gpu.queue, &[place(glam::Mat4::IDENTITY)]);
    let plain = render(&gpu, &renderer, &target, camera, NO_GRID);
    // Pull the top up by a metre: only the moved vertices are uploaded.
    let cube = mesh::PrimitiveKind::Cube.standard().mesh();
    let mut taller = cube.clone();
    let top: Vec<mesh::VertexId> = cube
        .vertices()
        .filter(|(_, p)| p.z > 50.0)
        .map(|(v, _)| v)
        .collect();
    for v in &top {
        let p = taller.position(*v).unwrap();
        taller.set_position(*v, p + Vec3::Z * 100.0);
    }
    let ranges = geometry.moved(&taller, &taller.moved_since(&cube).unwrap());
    renderer.update_mesh(&gpu.queue, 7, &geometry, &ranges);
    let high = Vec3::new(0.0, -50.0, 150.0);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(
        brightness(shot.at(high)) > brightness(plain.at(high)) + 60,
        "the cube is taller now"
    );
    // A mirrored placement looks the same as the plain one: the faces'
    // light doesn't turn inside out.
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[place(glam::Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)))],
    );
    let mirrored = render(&gpu, &renderer, &target, camera, NO_GRID);
    let front = Vec3::new(0.0, -50.0, 50.0);
    let difference = (brightness(mirrored.at(front)) - brightness(shot.at(front))).abs();
    assert!(difference < 30, "{difference}");
}

#[test]
fn an_object_can_show_every_point() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [640, 400]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let mut draw = place(glam::Mat4::IDENTITY);
    draw.edge_color = [0.2, 0.2, 0.2, 0.0];
    renderer.set_draws(&gpu.device, &gpu.queue, &[draw]);
    let without = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(!is_red(without.at(corner)));
    draw.points = Some(([1.0, 0.0, 0.0, 1.0], 10.0));
    renderer.set_draws(&gpu.device, &gpu.queue, &[draw]);
    let with = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert!(is_red(with.at(corner)), "{:?}", with.at(corner));
}

#[test]
fn without_faces_every_edge_shows_through_the_shape() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [640, 400]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    renderer.set_draws(&gpu.device, &gpu.queue, &[place(glam::Mat4::IDENTITY)]);
    let size = Vec2::new(640.0, 400.0);
    let red_pixels = |shot: &Shot| {
        shot.pixels
            .chunks(4)
            .filter(|p| is_red([p[0], p[1], p[2], p[3]]))
            .count()
    };
    assert!(renderer.shows_faces());
    let shaded = render(&gpu, &renderer, &target, camera, NO_GRID);
    let middle = Vec3::new(0.0, -50.0, 50.0);
    renderer.set_show_faces(false);
    let wire = render(&gpu, &renderer, &target, camera, NO_GRID);
    // The edges behind the cube show too: about half as many more.
    assert!(
        red_pixels(&wire) > red_pixels(&shaded) * 5 / 4,
        "{} vs {}",
        red_pixels(&wire),
        red_pixels(&shaded)
    );
    assert!(
        brightness(wire.at(middle)) < brightness(shaded.at(middle)) - 40,
        "no grey surface: the backdrop shows through"
    );
    // Picking still finds the shape.
    let pixel = camera.project(middle, size).unwrap();
    let picked = renderer
        .pick_now(
            &gpu.device,
            &gpu.queue,
            pick_request(&camera, pixel, [640, 400]),
        )
        .and_then(|r| r.point);
    assert!(picked.is_some());
}

#[test]
fn cavity_brightens_an_outer_edge_and_leaves_the_backdrop() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [640, 400]);
    with_object_cube(&gpu, &mut renderer);
    let mut draw = place(glam::Mat4::IDENTITY);
    // No edge lines, so only the shading shows.
    draw.edges = false;
    renderer.set_draws(&gpu.device, &gpu.queue, &[draw]);
    let camera = Camera::default();
    let size = Vec2::new(640.0, 400.0);
    let plain = render(&gpu, &renderer, &target, camera, NO_GRID);
    renderer.set_show_cavity(true);
    let cavity = render(&gpu, &renderer, &target, camera, NO_GRID);
    // Along the front-top edge (an outer corner: a ridge), the pixels just
    // around it are brighter.
    let edge = camera.project(Vec3::new(0.0, -50.0, 100.0), size).unwrap();
    let lift = |dy: i32| {
        let (x, y) = (edge.x as u32, (edge.y as i32 + dy) as u32);
        brightness(cavity.pixel(x, y)) - brightness(plain.pixel(x, y))
    };
    let most = (-3..=3).map(lift).max().unwrap();
    assert!(most > 20, "the ridge is lit: {most}");
    // The middle of the front face is flat: unchanged.
    let middle = Vec3::new(0.0, -50.0, 50.0);
    let flat = (brightness(cavity.at(middle)) - brightness(plain.at(middle))).abs();
    assert!(flat < 4, "{flat}");
    // The backdrop is untouched.
    let sky = (brightness(cavity.pixel(5, 5)) - brightness(plain.pixel(5, 5))).abs();
    assert_eq!(sky, 0);
    // Without faces (Wireframe) there's nothing to shade.
    renderer.set_show_faces(false);
    let wire = render(&gpu, &renderer, &target, camera, NO_GRID);
    renderer.set_show_cavity(false);
    let wire_plain = render(&gpu, &renderer, &target, camera, NO_GRID);
    assert_eq!(wire.pixels, wire_plain.pixels);
}

#[test]
fn the_checker_and_stretch_views_colour_the_surface_by_its_uvs() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    let geometry = with_object_cube(&gpu, &mut renderer);
    assert!(geometry.uv_density > 0.0, "the cube has UVs");
    let camera = Camera::default();
    let draw = |surface| render::MeshDraw {
        surface,
        ..place(glam::Mat4::IDENTITY)
    };
    // The front face, at two spots a quarter of the cube apart.
    let a = Vec3::new(-25.0, -50.0, 40.0);
    let b = Vec3::new(0.0, -50.0, 40.0);
    renderer.set_draws(&gpu.device, &gpu.queue, &[draw(render::SurfaceDraw::Plain)]);
    let plain = render(&gpu, &renderer, &target, camera, NO_GRID);
    let shade = brightness;
    assert!(
        (shade(plain.at(a)) - shade(plain.at(b))).abs() < 25,
        "an even grey"
    );
    // A checker of 16 squares across: neighbouring squares differ.
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[draw(render::SurfaceDraw::Checker { squares: 16.0 })],
    );
    let checker = render(&gpu, &renderer, &target, camera, NO_GRID);
    let spots: Vec<i32> = (0..8)
        .map(|i| shade(checker.at(Vec3::new(-40.0 + i as f32 * 10.0, -50.0, 40.0))))
        .collect();
    let (low, high) = (spots.iter().min().unwrap(), spots.iter().max().unwrap());
    assert!(high - low > 40, "light and dark squares: {spots:?}");
    // The stretch view: the cube's UVs are even and at its own density:
    // green.
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[draw(render::SurfaceDraw::Stretch {
            density: geometry.uv_density,
        })],
    );
    let even = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(even[1] > even[0] + 20, "green: {even:?}");
    // Compared with four times the density, it is far off: red.
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[draw(render::SurfaceDraw::Stretch {
            density: geometry.uv_density * 4.0,
        })],
    );
    let off = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(off[0] > off[1] + 20, "red: {off:?}");
    // The numbered checker (decision 0107): the image given, here one
    // plain blue, laid on by the UVs.
    let blue: Vec<u8> = (0..64 * 64).flat_map(|_| [20u8, 60, 230, 255]).collect();
    renderer.set_grid_image(&gpu.device, &gpu.queue, 64, &blue);
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[draw(render::SurfaceDraw::NumberedChecker)],
    );
    let image = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(
        image[2] > image[0] + 40 && image[2] > image[1] + 20,
        "blue: {image:?}"
    );
    // A new image is used at once, with the draws already given.
    let green: Vec<u8> = (0..64 * 64).flat_map(|_| [30u8, 220, 40, 255]).collect();
    renderer.set_grid_image(&gpu.device, &gpu.queue, 64, &green);
    let image = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(
        image[1] > image[0] + 40 && image[1] > image[2] + 40,
        "green: {image:?}"
    );
    // A wrongly sized image is ignored.
    renderer.set_grid_image(&gpu.device, &gpu.queue, 64, &green[..100]);
}

#[test]
fn painted_surfaces_show_each_slots_texture_over_its_colour() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    let a = Vec3::new(-25.0, -50.0, 40.0);
    let painted = |key: u64| {
        let mut textures = [0u64; render::PALETTE_SIZE];
        textures[0] = key;
        render::MeshDraw {
            surface: render::SurfaceDraw::Painted {
                textures,
                preview: render::PaintPreview::Lit,
            },
            ..place(glam::Mat4::IDENTITY)
        }
    };
    // No texture for its slot: its own grey, as Plain.
    renderer.set_draws(&gpu.device, &gpu.queue, &[painted(0)]);
    let bare = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(
        (bare[0] as i32 - bare[2] as i32).abs() < 20,
        "grey: {bare:?}"
    );
    // A red texture for slot 0.
    let red: Vec<u8> = (0..64 * 64).flat_map(|_| [230u8, 30, 20, 255]).collect();
    renderer.set_paint_texture(&gpu.device, &gpu.queue, 7, 64, &red);
    renderer.set_draws(&gpu.device, &gpu.queue, &[painted(7)]);
    let shown = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(shown[0] > shown[1] + 60, "red: {shown:?}");
    // Painted over with green: only the area given is written, here all.
    let green: Vec<u8> = (0..64 * 64).flat_map(|_| [30u8, 220, 40, 255]).collect();
    renderer.update_paint_texture(&gpu.queue, 7, 0, 0, 64, 64, &green);
    let shown = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(shown[1] > shown[0] + 60, "green: {shown:?}");
    // See-through paint leaves the colour under it.
    let clear = vec![0u8; 64 * 64 * 4];
    renderer.update_paint_texture(&gpu.queue, 7, 0, 0, 64, 64, &clear);
    let shown = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(
        (shown[0] as i32 - bare[0] as i32).abs() < 10,
        "{shown:?} {bare:?}"
    );
    // Areas that don't fit are ignored, and keys forgotten.
    renderer.update_paint_texture(&gpu.queue, 7, 60, 60, 10, 10, &green[..400]);
    assert_eq!(renderer.paint_texture_keys(), vec![7]);
    renderer.remove_paint_texture(7);
    assert!(renderer.paint_texture_keys().is_empty());
    // A buffer grown for more draws keeps painted slots drawable.
    renderer.set_paint_texture(&gpu.device, &gpu.queue, 7, 64, &red);
    let many: Vec<_> = (0..40).map(|_| painted(7)).collect();
    renderer.set_draws(&gpu.device, &gpu.queue, &many);
    let shown = render(&gpu, &renderer, &target, camera, NO_GRID).at(a);
    assert!(shown[0] > shown[1] + 60, "red again: {shown:?}");
}

#[test]
fn paint_previews_light_and_show_the_painting_their_own_way() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [320, 200]);
    with_object_cube(&gpu, &mut renderer);
    let camera = Camera::default();
    let a = Vec3::new(-25.0, -50.0, 40.0);
    let show = |renderer: &mut render::ViewportRenderer, preview| {
        let mut textures = [0u64; render::PALETTE_SIZE];
        textures[0] = 7;
        let draw = render::MeshDraw {
            surface: render::SurfaceDraw::Painted { textures, preview },
            ..place(glam::Mat4::IDENTITY)
        };
        renderer.set_draws(&gpu.device, &gpu.queue, &[draw]);
        render(&gpu, renderer, &target, camera, NO_GRID).at(a)
    };
    let red: Vec<u8> = (0..64 * 64).flat_map(|_| [230u8, 30, 20, 255]).collect();
    renderer.set_paint_texture(&gpu.device, &gpu.queue, 7, 64, &red);
    // Unlit: the painted colour itself.
    let unlit = show(&mut renderer, render::PaintPreview::Unlit);
    assert!(
        (unlit[0] as i32 - 230).abs() < 6 && (unlit[1] as i32 - 30).abs() < 6,
        "{unlit:?}"
    );
    let lit = show(&mut renderer, render::PaintPreview::Lit);
    assert_ne!(lit, unlit, "the studio light changes it");
    // Toon: the front faces the key light, so it is the lit band.
    let toon = show(&mut renderer, render::PaintPreview::Toon);
    assert_eq!(toon, unlit);
    // Godot: lit and tone mapped, still red.
    let godot = show(&mut renderer, render::PaintPreview::Godot);
    assert!(godot[0] > godot[1] + 60 && godot != unlit, "{godot:?}");
    let godot_toon = show(&mut renderer, render::PaintPreview::GodotToon);
    assert!(godot_toon[0] > godot_toon[1] + 60, "{godot_toon:?}");
    // Value check: a grey.
    let values = show(&mut renderer, render::PaintPreview::Values);
    assert!(
        values[0] == values[1] && values[1] == values[2],
        "{values:?}"
    );
    // Squint: a fine red and blue checker blurs into purple.
    let checker: Vec<u8> = (0..64 * 64)
        .flat_map(|i| {
            let (x, y) = (i % 64 / 4, i / 64 / 4);
            if (x + y) % 2 == 0 {
                [230u8, 20, 20, 255]
            } else {
                [20u8, 20, 230, 255]
            }
        })
        .collect();
    renderer.set_paint_texture(&gpu.device, &gpu.queue, 7, 64, &checker);
    let sharp = show(&mut renderer, render::PaintPreview::Unlit);
    let blurred = show(&mut renderer, render::PaintPreview::Squint);
    assert!(
        (sharp[0] as i32 - sharp[2] as i32).abs() > 120,
        "red or blue: {sharp:?}"
    );
    assert!(
        (blurred[0] as i32 - blurred[2] as i32).abs() < 60 && blurred[0] > 60,
        "purple: {blurred:?}"
    );
}

/// The improved bevel's rounded corners (decision 0141), drawn: a cube
/// with every edge rounded, seen from above one corner. The corner patch
/// is lit like the strips beside it, with no holes or dark folds. With
/// `DUMP_DIR` set, the picture is saved to look at.
#[test]
fn a_cube_with_rounded_corners_is_drawn_without_holes() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [480, 360]);
    renderer.set_shape(&gpu.device, &render::Shape::default());
    let mut cube = mesh::PrimitiveKind::Cube.standard().mesh();
    let edges = cube.edges().to_vec();
    mesh::bevel::bevel_edges(
        &mut cube,
        &edges,
        mesh::bevel::BevelShape {
            width: 20.0,
            segments: 6,
            ..Default::default()
        },
    );
    renderer.set_mesh(&gpu.device, 7, &render::MeshGeometry::new(&cube));
    renderer.set_draws(
        &gpu.device,
        &gpu.queue,
        &[render::MeshDraw {
            edges: false,
            ..place(glam::Mat4::IDENTITY)
        }],
    );
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let camera = Camera::looking_at(Vec3::new(260.0, -300.0, 330.0), Vec3::new(0.0, 0.0, 50.0));
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    // Across the rounded corner, from the top through the corner patch to
    // the sides: every pixel is the shape's, never the backdrop behind.
    let ball = Vec3::new(30.0, -30.0, 80.0);
    for k in 0..=20 {
        let t = k as f32 / 20.0;
        let towards = (corner - ball).normalize() + Vec3::new(t - 0.5, t - 0.5, 0.0) * 0.6;
        let on = ball + towards.normalize() * 19.0;
        let p = shot.at(on);
        assert!(brightness(p) > 3 * 0x50, "{on}: {p:?}");
    }
}

/// A remeshed shape (decision 0142), drawn: a torus rebuilt as even quads,
/// seen from above at an angle. Its surface faces out (lit, never the
/// dark inside), with no holes where the backdrop shows through. With
/// `DUMP_DIR` set, the picture is saved to look at.
#[test]
fn a_remeshed_shape_is_drawn_closed_and_facing_out() {
    let Some(gpu) = gpu() else { return };
    let (mut renderer, target) = setup(&gpu, [480, 360]);
    renderer.set_shape(&gpu.device, &render::Shape::default());
    let torus = mesh::PrimitiveKind::Torus.standard().mesh();
    let remeshed = mesh::remesh::remesh(
        &torus,
        &mesh::remesh::RemeshOptions {
            voxel_size: 4.0,
            smooth: 2,
        },
    )
    .unwrap();
    renderer.set_mesh(&gpu.device, 7, &render::MeshGeometry::new(&remeshed));
    renderer.set_draws(&gpu.device, &gpu.queue, &[place(glam::Mat4::IDENTITY)]);
    let bounds = remeshed.bounds().unwrap();
    let centre = bounds.centre();
    let camera = Camera::looking_at(centre + Vec3::new(120.0, -160.0, 180.0), centre);
    let shot = render(&gpu, &renderer, &target, camera, NO_GRID);
    // Round the ring's top, half way between its inner and outer edges:
    // every pixel is the shape's, lit.
    let size = bounds.size();
    let (outer, top) = (size.x / 2.0, bounds.max.z);
    let tube = size.z / 2.0;
    let middle = outer - tube;
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        let on = Vec3::new(
            centre.x + middle * a.cos(),
            centre.y + middle * a.sin(),
            top,
        );
        let p = shot.at(on);
        assert!(brightness(p) > 3 * 0x50, "{on}: {p:?}");
    }
}
