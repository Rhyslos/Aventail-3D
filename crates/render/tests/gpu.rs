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
    render_with(gpu, renderer, target, camera, focus, false)
}

fn render_with(
    gpu: &Gpu,
    renderer: &ViewportRenderer,
    target: &ViewportTarget,
    camera: Camera,
    focus: Vec3,
    fine_grid: bool,
) -> Shot {
    let size = target.size();
    let params = FrameParams {
        view_projection: camera.view_projection(Vec2::new(size[0] as f32, size[1] as f32)),
        camera_position: camera.position,
        focus,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        fine_grid,
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
fn fine_grid_lines_show_only_when_switched_on() {
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
        true,
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
        false,
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
