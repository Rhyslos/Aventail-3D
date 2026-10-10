//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. CI sets it in the window test job, which has a software
//! Vulkan driver, so these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use glam::{Vec2, Vec3};
use render::{
    FrameParams, TEST_CUBE_SIZE, ViewportRenderer, ViewportTarget, best_sample_count, read_pixels,
};
use tools::Camera;

/// The viewport gradient from the interface theme, sRGB 0 to 1.
const SKY_TOP: [f32; 3] = [92.0 / 255.0, 95.0 / 255.0, 100.0 / 255.0];
const SKY_BOTTOM: [f32; 3] = [57.0 / 255.0, 59.0 / 255.0, 63.0 / 255.0];

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

fn render(gpu: &Gpu, renderer: &ViewportRenderer, target: &ViewportTarget, camera: Camera) -> Shot {
    let size = target.size();
    let params = FrameParams {
        view_projection: camera.view_projection(Vec2::new(size[0] as f32, size[1] as f32)),
        camera_position: camera.position,
        sky_top: SKY_TOP,
        sky_bottom: SKY_BOTTOM,
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

#[test]
fn the_background_is_the_calm_gradient() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [320, 200]);
    // Looking up at the sky, nothing but background shows.
    let camera = Camera::looking_at(Vec3::new(0.0, 0.0, 100.0), Vec3::new(0.0, 1.0, 200.0));
    let shot = render(&gpu, &renderer, &target, camera);
    let top = shot.pixel(160, 1);
    let bottom = shot.pixel(160, 198);
    for (got, want) in [(top, [92, 95, 100]), (bottom, [57, 59, 63])] {
        for c in 0..3 {
            assert!(
                (i32::from(got[c]) - want[c]).abs() <= 3,
                "{got:?} should be about {want:?}"
            );
        }
        assert_eq!(got[3], 255);
    }
}

#[test]
fn the_cube_is_shaded_like_a_studio_light() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    let shot = render(&gpu, &renderer, &target, Camera::default());
    let h = TEST_CUBE_SIZE / 2.0;
    let top = shot.at(Vec3::new(0.0, 0.0, TEST_CUBE_SIZE));
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
fn the_x_axis_is_red_and_the_y_axis_green() {
    let Some(gpu) = gpu() else { return };
    let (renderer, target) = setup(&gpu, [640, 400]);
    // From above, so the axes are easy to hit and the cube is out of the way.
    let camera = Camera::looking_at(
        Vec3::new(-600.0, -600.0, 900.0),
        Vec3::new(-300.0, -300.0, 0.0),
    );
    let shot = render(&gpu, &renderer, &target, camera);
    let size = Vec2::new(640.0, 400.0);
    let redness = |p: [u8; 4]| i32::from(p[0]) - i32::from(p[1]);
    let greenness = |p: [u8; 4]| i32::from(p[1]) - i32::from(p[0]);
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
        best(Vec3::new(0.0, -400.0, 0.0), &greenness) > 15,
        "Y axis not green"
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
    let shot = render(&gpu, &renderer, &target, camera);
    // x = -400 is a 1 m line; x = -455, y = -255 is between all lines.
    let on_line = shot.at(Vec3::new(-400.0, -255.0, 0.0));
    let between = shot.at(Vec3::new(-455.0, -255.0, 0.0));
    assert!(
        brightness(on_line) > brightness(between) + 6,
        "{on_line:?} vs {between:?}"
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
    let shot = render(&gpu, &renderer, &target, Camera::default());
    assert_eq!(shot.pixels.len(), 300 * 150 * 4);
}
