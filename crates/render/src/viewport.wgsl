// Viewport shaders: the background gradient, the ground grid, coloured
// lines (axes and edges) and shaded faces.
//
// Colours are written in linear space; the render target is sRGB, so the
// GPU converts them when storing. Z is up; the grid lies on the X-Y plane.

struct Frame {
    // World to clip space.
    view_proj: mat4x4<f32>,
    // Camera position (xyz); w unused.
    camera: vec4<f32>,
    // Grid spacings: minor, major, super; w = half the grid's width.
    grid: vec4<f32>,
    // Opacity of each grid level (minor, major, super; w unused). The
    // minor level is 0 when fine grid lines are switched off.
    grid_strength: vec4<f32>,
    // Background gradient, linear colour: top of the view, then bottom.
    sky_top: vec4<f32>,
    sky_bottom: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;

// ---------------------------------------------------------- background ---

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    // 0 at the top of the view, 1 at the bottom.
    @location(0) down: f32,
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
    out.down = (1.0 - xy.y) * 0.5;
    return out;
}

@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    let t = clamp(in.down, 0.0, 1.0);
    return vec4<f32>(mix(frame.sky_top.rgb, frame.sky_bottom.rgb, t), 1.0);
}

// ---------------------------------------------------------------- grid ---

struct GridOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
};

// One large square on the ground (Z = 0), drawn as two triangles. The grid
// lines themselves are computed per pixel in the fragment shader.
@vertex
fn grid_vs(@builtin(vertex_index) index: u32) -> GridOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let xy = corners[index] * frame.grid.w;
    var out: GridOut;
    out.clip = frame.view_proj * vec4<f32>(xy, 0.0, 1.0);
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

    // Fade softly with distance, farther when the camera is higher, so the
    // grid never ends in a hard edge.
    let height = abs(frame.camera.z);
    let reach = max(2000.0, height * 6.0);
    let distance = length(in.world - frame.camera.xy);
    let distance_fade = 1.0 - smoothstep(reach * 0.35, reach, distance);

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
    let on_ground = select(1.0, 0.0, abs(in.world.z) > 0.001);
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
@fragment
fn face_fs(in: FaceOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let to_camera = normalize(frame.camera.xyz - in.world);

    let key = normalize(vec3<f32>(-0.3, -0.35, 0.9));
    let wrap = 0.5;
    let diffuse = max((dot(n, key) + wrap) / (1.0 + wrap), 0.0);
    let fill = mix(0.36, 0.50, n.z * 0.5 + 0.5);
    let facing = max(dot(n, to_camera), 0.0);
    let headlight = 0.12 * facing;
    let rim = 0.06 * pow(1.0 - facing, 3.0);
    let contact = mix(0.80, 1.0, smoothstep(0.0, 80.0, in.world.z));

    let shade = (fill + 0.5 * diffuse + headlight) * contact + rim;
    return vec4<f32>(in.color.rgb * shade, in.color.a);
}
