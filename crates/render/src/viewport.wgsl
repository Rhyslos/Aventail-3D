// Viewport shaders: the background gradient, the ground grid, coloured
// lines (axes and edges) and shaded faces.
//
// Colours are written in linear space; the render target is sRGB, so the
// GPU converts them when storing. Z is up; the grid lies on the X-Y plane.

struct Frame {
    // World to clip space.
    view_proj: mat4x4<f32>,
    // Clip space back to world space, to find each pixel's view direction.
    inv_view_proj: mat4x4<f32>,
    // Camera position (xyz); w unused.
    camera: vec4<f32>,
    // The point the camera looks at (xyz); w = how far around it the grid
    // reaches before it has faded out.
    focus: vec4<f32>,
    // Grid spacings: minor, major, super; w = half the grid's width.
    grid: vec4<f32>,
    // Opacity of each grid level (minor, major, super; w unused). The
    // minor level is 0 when fine grid lines are switched off.
    grid_strength: vec4<f32>,
    // The two directions the grid's plane spans (xyz; w unused): X and Y
    // for the ground, or an upright plane facing a flat side view.
    grid_u: vec4<f32>,
    grid_v: vec4<f32>,
    // x = 1 for a flat (orthographic) view, 0 for perspective.
    view: vec4<f32>,
    // Background, linear colour: the sky straight up, the sky at the
    // horizon, and the ground just below the horizon.
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    ground: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;

// ---------------------------------------------------------- background ---

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    // Position on screen, -1 to 1 both ways.
    @location(0) ndc: vec2<f32>,
};

// One triangle that covers the whole view.
@vertex
fn sky_vs(@builtin(vertex_index) index: u32) -> SkyOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[index];
    var out: SkyOut;
    out.clip = vec4<f32>(xy, 1.0, 1.0);
    out.ndc = xy;
    return out;
}

// A calm studio backdrop with a horizon: the sky lightens gently towards
// the top, and below the horizon lies a slightly darker ground that
// darkens a little more towards your feet. The horizon follows the camera
// as it tilts, so you always know which way is up.
@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    // Flat views have no horizon: a plain, calm backdrop.
    if frame.view.x > 0.5 {
        return vec4<f32>(frame.sky_horizon.rgb, 1.0);
    }
    let far = frame.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(far.xyz / far.w - frame.camera.xyz);
    let elevation = direction.z;
    let sky = mix(frame.sky_horizon.rgb, frame.sky_top.rgb, smoothstep(0.0, 0.7, elevation));
    let ground = frame.ground.rgb * mix(1.0, 0.8, smoothstep(0.0, 0.7, -elevation));
    // Soften the horizon over about one pixel, so it never looks jagged.
    let edge = max(fwidth(elevation), 1e-5);
    let above = smoothstep(-edge, edge, elevation);
    return vec4<f32>(mix(ground, sky, above), 1.0);
}

// ---------------------------------------------------------------- grid ---

struct GridOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
};

// One large square on the grid's plane (the ground, or an upright plane
// facing a flat side view), drawn as two triangles. The grid lines
// themselves are computed per pixel in the fragment shader.
@vertex
fn grid_vs(@builtin(vertex_index) index: u32) -> GridOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let xy = corners[index] * frame.grid.w;
    let world = frame.grid_u.xyz * xy.x + frame.grid_v.xyz * xy.y;
    var out: GridOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    out.world = xy;
    return out;
}

// How strongly a pixel lies on a grid line of the given spacing (0 to 1).
// Lines are one pixel wide whatever the distance, and a level fades out
// when its lines would be closer together than a few pixels, which avoids
// shimmering patterns in the distance.
fn grid_line(coord: vec2<f32>, spacing: f32) -> f32 {
    let cell = coord / spacing;
    let per_pixel = max(fwidth(cell), vec2<f32>(1e-6, 1e-6));
    let distance_to_line = abs(fract(cell - 0.5) - 0.5) / per_pixel;
    let on_line = 1.0 - min(min(distance_to_line.x, distance_to_line.y), 1.0);
    let density = max(per_pixel.x, per_pixel.y);
    let level_fade = 1.0 - smoothstep(0.08, 0.25, density);
    return on_line * level_fade;
}

@fragment
fn grid_fs(in: GridOut) -> @location(0) vec4<f32> {
    let minor = grid_line(in.world, frame.grid.x);
    let major = grid_line(in.world, frame.grid.y);
    let super_ = grid_line(in.world, frame.grid.z);

    // A finite grid around the point the camera looks at, fading softly
    // to nothing, so it reads as a work surface rather than an endless
    // floor. Zooming out widens it.
    let reach = frame.focus.w;
    let focus = vec2<f32>(dot(frame.focus.xyz, frame.grid_u.xyz), dot(frame.focus.xyz, frame.grid_v.xyz));
    let distance = length(in.world - focus);
    let distance_fade = 1.0 - smoothstep(reach * 0.25, reach, distance);

    let strength = frame.grid_strength;
    let alpha = max(max(minor * strength.x, major * strength.y), super_ * strength.z) * distance_fade;
    if alpha <= 0.002 {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, alpha);
}

// --------------------------------------------------------------- lines ---

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn line_vs(in: LineIn) -> LineOut {
    var out: LineOut;
    out.clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.world = in.position;
    return out;
}

@fragment
fn line_fs(in: LineOut) -> @location(0) vec4<f32> {
    // Lines on the ground fade with distance like the grid.
    let height = abs(frame.camera.z);
    let reach = max(2000.0, height * 6.0);
    let distance = length(in.world.xy - frame.camera.xy);
    // Flat views show the axes in full.
    let on_ground = select(1.0, 0.0, abs(in.world.z) > 0.001 || frame.view.x > 0.5);
    let fade = 1.0 - on_ground * smoothstep(reach * 0.35, reach, distance);
    return vec4<f32>(in.color.rgb, in.color.a * fade);
}

// --------------------------------------------------------------- faces ---

struct FaceIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct FaceOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn face_vs(in: FaceIn) -> FaceOut {
    var out: FaceOut;
    out.clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.world = in.position;
    return out;
}

// Soft studio lighting, tuned to feel calm rather than crisp:
// - a sky-and-ground fill light, so no side is ever dark;
// - one key light from the front, above and to the left, "wrapped" so its
//   light bends gently past the edges and neighbouring sides differ softly;
// - a faint light from the camera, so the shade drifts gently across each
//   face instead of every face being one flat colour;
// - a faint brightening where a surface turns away from view, which
//   rounds off silhouettes;
// - slightly darker shading close to the ground, as if the floor blocks a
//   little light, so shapes sit on the grid instead of floating.
//
// Faces are drawn from both sides; seen from behind (the underside of a
// plane, the inside of an open cylinder) the light treats the back as the
// front, a little darker, so the inside of a shape still reads as inside.
@fragment
fn face_fs(in: FaceOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let n = select(-normalize(in.normal), normalize(in.normal), front);
    let back = select(0.8, 1.0, front);
    let to_camera = normalize(frame.camera.xyz - in.world);

    let key = normalize(vec3<f32>(-0.3, -0.35, 0.9));
    let wrap = 0.5;
    let diffuse = max((dot(n, key) + wrap) / (1.0 + wrap), 0.0);
    let fill = mix(0.36, 0.50, n.z * 0.5 + 0.5);
    let facing = max(dot(n, to_camera), 0.0);
    let headlight = 0.12 * facing;
    let rim = 0.06 * pow(1.0 - facing, 3.0);
    let contact = mix(0.80, 1.0, smoothstep(0.0, 80.0, in.world.z));

    let shade = ((fill + 0.5 * diffuse + headlight) * contact + rim) * back;
    return vec4<f32>(in.color.rgb * shade, in.color.a);
}
