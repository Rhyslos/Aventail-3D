// Viewport shaders: the background gradient, the ground grid, coloured
// lines (axes and edges), shaded faces, the overlay, and objects' meshes
// placed in the world.
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
    // Which way is right and up on screen, in the world (xyz; w unused).
    screen_right: vec4<f32>,
    screen_up: vec4<f32>,
    // The selection's outline (decision 0220): its colour (rgb, linear)
    // and width in pixels (w; 0 draws none).
    selection: vec4<f32>,
    // x: how wide objects' edges are drawn, in pixels (decision 0221);
    // yzw unused.
    lines: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;

// What the first stage writes: the colour, and which way the surface
// faces across the screen for cavity (xy: right and up, z: towards the
// camera; w: 1 on a surface, 2 on a selected object's surface for its
// outline, 0 elsewhere).
struct SceneOut {
    @location(0) color: vec4<f32>,
    @location(1) facing: vec4<f32>,
};

// A surface's facing across the screen, for `normal` in the world.
fn screen_facing(normal: vec3<f32>) -> vec4<f32> {
    let n = normalize(normal);
    let toward = cross(frame.screen_right.xyz, frame.screen_up.xyz);
    return vec4<f32>(dot(n, frame.screen_right.xyz), dot(n, frame.screen_up.xyz), dot(n, toward), 1.0);
}

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
fn sky_fs(in: SkyOut) -> SceneOut {
    var out: SceneOut;
    out.color = sky_color(in);
    out.facing = vec4<f32>(0.0);
    return out;
}

fn sky_color(in: SkyOut) -> vec4<f32> {
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
    // The UV, for the checker and stretch views (objects' meshes only).
    @location(3) uv: vec2<f32>,
    // The vertex colour (decision 0205), sRGB with alpha; white elsewhere.
    @location(4) vertex_colour: vec4<f32>,
};

@vertex
fn face_vs(in: FaceIn) -> FaceOut {
    var out: FaceOut;
    out.clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.world = in.position;
    out.uv = vec2<f32>(0.0);
    out.vertex_colour = vec4<f32>(1.0);
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
fn face_fs(in: FaceOut, @builtin(front_facing) front: bool) -> SceneOut {
    var out: SceneOut;
    out.color = shade(in, front);
    out.facing = screen_facing(select(-in.normal, in.normal, front));
    return out;
}

fn shade(in: FaceOut, front: bool) -> vec4<f32> {
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

// ------------------------------------------------------------- overlay ---

// Points and thick lines for the Points, Edges and Polygons modes, a fixed
// number of pixels wide whatever the distance. Each is one record (a point
// has both ends in the same place) drawn as two triangles, six corners
// worked out here and pushed out on screen from the point (or the line's
// ends). Points are cut round in the fragment shader, with a soft
// one-pixel rim. frame.view.yz holds the image's size in pixels.

struct SpriteIn {
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    @location(2) width: f32,
    @location(3) color: vec4<f32>,
};

struct SpriteOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Where in the square this is (-1 to 1 each way), for cutting points
    // round.
    @location(1) corner: vec2<f32>,
    // 1 for a point (cut round), 0 for a line.
    @location(2) round: f32,
    // One pixel, in corner units.
    @location(3) pixel: f32,
};

@vertex
fn sprite_vs(@builtin(vertex_index) index: u32, in: SpriteIn) -> SpriteOut {
    let i = index % 6u;
    var point_corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    // For a line: which end each corner is at (1 for b), and which side.
    // Each end measures "sideways" from its own direction along the line,
    // so side +1 at b is the same side as -1 at a.
    var at_b = array<f32, 6>(0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
    var sides = array<f32, 6>(-1.0, 1.0, -1.0, -1.0, -1.0, 1.0);
    let is_line = any(in.a != in.b);
    var position = in.a;
    var other_end = in.b;
    var corner = point_corners[i];
    if is_line {
        if at_b[i] > 0.5 {
            position = in.b;
            other_end = in.a;
        }
        corner = vec2<f32>(-1.0, sides[i]);
    }
    let size = max(frame.view.yz, vec2<f32>(1.0, 1.0));
    var clip = frame.view_proj * vec4<f32>(position, 1.0);
    // Half the width, in pixels.
    let half = in.width * 0.5;
    var offset = corner * half;
    var round = 1.0;
    if is_line {
        round = 0.0;
        // A line: push sideways from the line on screen, and a little past
        // this end so joined lines meet without gaps.
        let other = frame.view_proj * vec4<f32>(other_end, 1.0);
        let here_px = clip.xy / max(clip.w, 1e-5) * size * 0.5;
        let there_px = other.xy / max(other.w, 1e-5) * size * 0.5;
        var along = there_px - here_px;
        if length(along) < 1e-4 {
            along = vec2<f32>(1.0, 0.0);
        }
        let dir = normalize(along);
        let side = vec2<f32>(-dir.y, dir.x);
        offset = side * corner.y * half + dir * corner.x * half;
    }
    clip = vec4<f32>(clip.xy + offset * 2.0 / size * clip.w, clip.z, clip.w);
    var out: SpriteOut;
    out.clip = clip;
    out.color = in.color;
    out.corner = corner;
    out.round = round;
    out.pixel = 2.0 / max(in.width, 1.0);
    return out;
}

@fragment
fn sprite_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    if in.round < 0.5 {
        return in.color;
    }
    let edge = 1.0 - smoothstep(1.0 - in.pixel, 1.0, length(in.corner));
    if edge <= 0.0 {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * edge);
}

// ------------------------------------------------------------- objects ---

// Each object's mesh stays on the GPU in its own space; these say where
// it is and how to draw it this frame (see mesh_draw.rs).
struct Object {
    model: mat4x4<f32>,
    // The model matrix's inverse transposed, for normals.
    normal: mat4x4<f32>,
    edge_color: vec4<f32>,
    point_color: vec4<f32>,
    // x: point size in pixels; y: -1 when the placement mirrors the shape;
    // z: the average edge length in the world; w: how faint a dense
    // wireframe may fade.
    options: vec4<f32>,
    // What the surface shows (decision 0102): x: 5 vertex colours
    // (decision 0205), 4 painted (decision 0177), 0 its colours, 1 a
    // checker, 2 how its UVs stretch, 3 the numbered checker image
    // (decision 0107); y: checker squares across the
    // texture; z: the texel density (texture widths per world centimetre)
    // the stretch view compares with; w: for painted, the preview
    // (decision 0198): 0 lit, 1 unlit, 2 toon, 3 Godot, 4 values, 5 squint,
    // 6 Godot toon, 7 realistic, 8 roughness, 9 metal (decision 0207);
    // for painted, y: how far the realistic light is turned (radians).
    surface: vec4<f32>,
    // The outline (decision 0201): colour (rgb, linear) and width in
    // centimetres (w).
    outline: vec4<f32>,
    // x: 1 for a selected object, outlined (decision 0220).
    marks: vec4<f32>,
    // The material colours by slot, four to a row, each sRGB bytes (red
    // lowest) with 255 above; 0 for the standard surface colour.
    materials: array<vec4<u32>, 16>,
};

@group(1) @binding(0) var<uniform> object: Object;
// The numbered checker image (decision 0107), its first row the top of
// the texture square, and how it is read (repeating, smoothed, with
// smaller copies for far away).
@group(1) @binding(1) var grid_image: texture_2d<f32>;
@group(1) @binding(2) var grid_sampler: sampler;
// A painted surface's roughness (red) and metal (green), linear, with
// alpha 1 where the texture set has them, and its glow (decision 0207).
@group(1) @binding(3) var surface_image: texture_2d<f32>;
@group(1) @binding(4) var glow_image: texture_2d<f32>;

// The shapes' surface colour, linear space (geometry.rs CUBE_COLOR).
const SHAPE_COLOR = vec4<f32>(0.5, 0.5, 0.51, 1.0);

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) material: u32,
    @location(3) uv: vec2<f32>,
    @location(4) colour: vec4<f32>,
};

// An sRGB colour as linear light.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// The surface colour of material slot `slot` (decision 0074).
fn material_color(slot: u32) -> vec4<f32> {
    let s = min(slot, 63u);
    let packed = object.materials[s / 4u][s % 4u];
    if packed == 0u {
        return SHAPE_COLOR;
    }
    return vec4<f32>(srgb_to_linear(unpack4x8unorm(packed).rgb), 1.0);
}

fn object_world(position: vec3<f32>) -> vec3<f32> {
    return (object.model * vec4<f32>(position, 1.0)).xyz;
}

@vertex
fn mesh_face_vs(in: MeshIn) -> FaceOut {
    let world = object_world(in.position);
    var out: FaceOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    out.normal = (object.normal * vec4<f32>(in.normal, 0.0)).xyz;
    out.color = material_color(in.material);
    out.world = world;
    out.uv = in.uv;
    out.vertex_colour = in.colour;
    return out;
}

// The stretch view's colours: even, a little off, far off.
const EVEN = vec3<f32>(0.20, 0.55, 0.36);
const SOME = vec3<f32>(0.85, 0.70, 0.18);
const FAR = vec3<f32>(0.80, 0.18, 0.16);
// A surface with no room on the texture (no UVs, or squashed flat).
const NO_ROOM = vec3<f32>(0.55, 0.25, 0.60);

// The colour the surface shows (decision 0102): its own, a checker laid on
// by its UVs (tinted across the square so turns and mirrors show), or how
// much its UVs stretch here. The stretch is read from how the UV and the
// world position change across the pixel: their ratio is the map from the
// surface to the texture, whose two stretches (largest and smallest) say
// how unevenly (their ratio) and how big (their product, against the
// object's texel density) the texture lies.
fn surface_color(in: FaceOut) -> vec4<f32> {
    let dpx = dpdx(in.world);
    let dpy = dpdy(in.world);
    let dux = dpdx(in.uv);
    let duy = dpdy(in.uv);
    let mode = object.surface.x;
    if mode < 0.5 {
        return in.color;
    }
    if mode > 4.5 {
        // Vertex colours (decision 0205), laid over the material's colour
        // by their alpha.
        let v = in.vertex_colour;
        return vec4<f32>(mix(in.color.rgb, srgb_to_linear(v.rgb), v.a), in.color.a);
    }
    if mode > 3.5 {
        // Painted (decision 0177): this material slot's texture, over its
        // colour where the paint lets it through.
        // Squinting reads a smaller copy: about 16 times blurrier.
        let blur = select(1.0, 16.0, abs(object.surface.w - 5.0) < 0.5);
        let flip = vec2<f32>(1.0, -1.0) * blur;
        let at = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
        let c = textureSampleGrad(grid_image, grid_sampler, at, dux * flip, duy * flip);
        return vec4<f32>(mix(in.color.rgb, c.rgb, c.a), in.color.a);
    }
    if mode > 2.5 {
        // The image's first row is the top of the square (V = 1).
        let flip = vec2<f32>(1.0, -1.0);
        let at = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
        let c = textureSampleGrad(grid_image, grid_sampler, at, dux * flip, duy * flip);
        return vec4<f32>(c.rgb, 1.0);
    }
    if mode < 1.5 {
        let squares = max(object.surface.y, 1.0);
        let cell = vec2<i32>(floor(in.uv * squares));
        let odd = ((cell.x + cell.y) % 2 + 2) % 2;
        let light = select(0.82, 0.30, odd == 1);
        let t = fract(in.uv);
        let tint = vec3<f32>(0.62 + 0.38 * t.x, 0.62 + 0.38 * t.y, 0.78);
        return vec4<f32>(tint * light, 1.0);
    }
    let across = cross(dpx, dpy);
    if length(dpx) < 1e-12 || length(across) < 1e-20 {
        return vec4<f32>(NO_ROOM, 1.0);
    }
    let t = normalize(dpx);
    let b = normalize(cross(normalize(across), t));
    let px = vec2<f32>(length(dpx), 0.0);
    let py = vec2<f32>(dot(dpy, t), dot(dpy, b));
    let det = px.x * py.y - py.x * px.y;
    if abs(det) < 1e-20 {
        return vec4<f32>(NO_ROOM, 1.0);
    }
    let inverse = mat2x2<f32>(vec2<f32>(py.y, -px.y), vec2<f32>(-py.x, px.x)) * (1.0 / det);
    let j = mat2x2<f32>(dux, duy) * inverse;
    let a = transpose(j) * j;
    let trace = a[0][0] + a[1][1];
    let d = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    let spread = sqrt(max(trace * trace / 4.0 - d, 0.0));
    let most = sqrt(max(trace / 2.0 + spread, 0.0));
    let least = sqrt(max(trace / 2.0 - spread, 0.0));
    if most < 1e-12 {
        return vec4<f32>(NO_ROOM, 1.0);
    }
    let uneven = log2(most / max(least, most * 1e-4));
    let size = sqrt(most * least) / max(object.surface.z, 1e-20);
    let off = max(uneven, abs(log2(max(size, 1e-6))));
    // 1.5 times off is SOME; twice off is FAR.
    let half = 0.585;
    var c: vec3<f32>;
    if off < half {
        c = mix(EVEN, SOME, off / half);
    } else {
        c = mix(SOME, FAR, clamp((off - half) / (1.0 - half), 0.0, 1.0));
    }
    return vec4<f32>(c, 1.0);
}

// A mirrored placement turns the faces inside out: what the GPU sees as
// the back is the outside.
@fragment
fn mesh_face_fs(in: FaceOut, @builtin(front_facing) front: bool) -> SceneOut {
    let outside = front == (object.options.y > 0.0);
    var surface = in;
    surface.color = surface_color(in);
    var out: SceneOut;
    if object.surface.x > 3.5 && object.surface.x < 4.5 {
        // The realism channels (decision 0207), where painted.
        let at = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
        let s = textureSample(surface_image, grid_sampler, at);
        let g = textureSample(glow_image, grid_sampler, at);
        let realism = Realism(mix(0.8, s.r, s.a), s.g * s.a, g.rgb * g.a, mix(0.5, s.b, s.a));
        // The height channel's bumps (decision 0211): the surface turned
        // by how fast the height changes across the pixel (bump mapping
        // from screen-space changes, needing no tangents).
        let h = mix(0.5, s.b, s.a);
        let dhx = dpdx(h) * object.surface.z;
        let dhy = dpdy(h) * object.surface.z;
        if object.surface.z > 0.0 {
            let dpx = dpdx(in.world);
            let dpy = dpdy(in.world);
            let n0 = normalize(in.normal);
            let r1 = cross(dpy, n0);
            let r2 = cross(n0, dpx);
            let det = dot(dpx, r1);
            let grad = sign(det) * (dhx * r1 + dhy * r2);
            let bumped = abs(det) * n0 - grad;
            if length(bumped) > 1e-12 {
                surface.normal = normalize(bumped);
            }
        }
        if object.surface.w > 0.5 {
            out.color = preview(surface, outside, realism);
        } else {
            out.color = shade(surface, outside);
        }
    } else if object.surface.x > 3.5 && object.surface.w > 0.5 {
        out.color = preview(surface, outside, Realism(0.8, 0.0, vec3<f32>(0.0), 0.5));
    } else {
        out.color = shade(surface, outside);
    }
    out.facing = screen_facing(select(-in.normal, in.normal, outside));
    // A selected object's surface, for its outline (decision 0220).
    out.facing.w = select(1.0, 2.0, object.marks.x > 0.5);
    return out;
}

// A painted surface's realism channels (decision 0207): roughness and
// metal 0 to 1, glow linear light.
struct Realism {
    roughness: f32,
    metal: f32,
    glow: vec3<f32>,
    // The height, 0 to 1, 0.5 flat (decision 0211).
    height: f32,
};

// The sky the realistic preview is lit and reflects: warm ground, a
// light horizon, a blue sky above.
fn sky_light(dir: vec3<f32>) -> vec3<f32> {
    let ground = vec3<f32>(0.20, 0.18, 0.16);
    let horizon = vec3<f32>(0.70, 0.72, 0.74);
    let zenith = vec3<f32>(0.30, 0.45, 0.75);
    if dir.z < 0.0 {
        return mix(horizon, ground, smoothstep(0.0, 0.3, -dir.z));
    }
    return mix(horizon, zenith, smoothstep(0.0, 0.6, dir.z));
}

// Realistic light (decision 0207): a sun turned `turn` radians round the
// model and the sky, with the standard microfacet model engines use
// (GGX), then Filmic tone mapping.
fn realistic(in: FaceOut, n: vec3<f32>, c: vec3<f32>, r: Realism) -> vec3<f32> {
    let pi = 3.14159265;
    let turn = object.surface.y;
    let base = vec3<f32>(-0.433, -0.25, 0.866);
    let sun = normalize(vec3<f32>(
        base.x * cos(turn) - base.y * sin(turn),
        base.x * sin(turn) + base.y * cos(turn),
        base.z,
    ));
    let v = normalize(frame.camera.xyz - in.world);
    let rough = clamp(r.roughness, 0.04, 1.0);
    let metal = clamp(r.metal, 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), c, metal);
    let n_v = max(dot(n, v), 1e-4);
    // The sun.
    let h = normalize(sun + v);
    let n_l = max(dot(n, sun), 0.0);
    let n_h = max(dot(n, h), 0.0);
    let v_h = max(dot(v, h), 0.0);
    let a2 = pow(rough, 4.0);
    let d = a2 / (pi * pow(n_h * n_h * (a2 - 1.0) + 1.0, 2.0));
    let k = pow(rough + 1.0, 2.0) / 8.0;
    let g = (n_v / (n_v * (1.0 - k) + k)) * (n_l / (n_l * (1.0 - k) + k));
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_h, 5.0);
    let specular = d * g * f / max(4.0 * n_v * n_l, 1e-4);
    let diffuse = (vec3<f32>(1.0) - f) * (1.0 - metal) * c / pi;
    let direct = (diffuse + specular) * n_l * 3.0;
    // The sky: its light from around, and its reflection, blurred toward
    // its average on rough surfaces.
    let ambient = c * (1.0 - metal) * sky_light(n) * 0.9;
    let reflected = reflect(-v, n);
    let average = vec3<f32>(0.45, 0.48, 0.52);
    let env = mix(sky_light(reflected), average, rough * rough);
    let fr = f0 + (max(vec3<f32>(1.0 - rough), f0) - f0) * pow(1.0 - n_v, 5.0);
    return direct + ambient + env * fr + r.glow;
}

// The paint previews other than the studio light (decision 0198).
fn preview(in: FaceOut, front: bool, realism: Realism) -> vec4<f32> {
    let n = select(-normalize(in.normal), normalize(in.normal), front);
    let c = in.color.rgb;
    let mode = object.surface.w;
    if mode > 10.5 {
        // Only the glow, on black (decision 0213).
        return vec4<f32>(realism.glow, in.color.a);
    }
    if mode > 9.5 {
        // The height as greys (decision 0213).
        return vec4<f32>(vec3<f32>(realism.height), in.color.a);
    }
    if mode > 7.5 {
        // Roughness or metal as greys (decision 0207).
        let value = select(realism.roughness, realism.metal, mode > 8.5);
        return vec4<f32>(vec3<f32>(value), in.color.a);
    }
    if mode > 6.5 {
        return vec4<f32>(filmic(realistic(in, n, c, realism)), in.color.a);
    }
    // The key light the studio light uses: front, above, left.
    let key = normalize(vec3<f32>(-0.3, -0.35, 0.9));
    if mode < 1.5 || abs(mode - 5.0) < 0.5 {
        // Unlit, and squint: the colours as painted.
        return vec4<f32>(c, in.color.a);
    }
    if mode < 2.5 {
        // Toon: lit, or shaded a little cooler, with a hard edge, and a
        // thin rim of light where the surface turns away from view.
        let lit = step(0.15, dot(n, key));
        let shaded = c * vec3<f32>(0.50, 0.53, 0.62);
        let to_camera = normalize(frame.camera.xyz - in.world);
        let rim = smoothstep(0.62, 0.68, 1.0 - max(dot(n, to_camera), 0.0)) * 0.3;
        return vec4<f32>(mix(shaded, c, lit) + vec3<f32>(rim), in.color.a);
    }
    if mode < 3.5 || mode > 5.5 {
        // Godot's standard look: a sun of strength 1, the default sky's
        // light from above and around, then Filmic tone mapping. Its
        // toon diffuse (6) turns the sun's light into a step, softened by
        // a roughness of 0.1.
        let sun = normalize(vec3<f32>(-0.433, -0.25, 0.866));
        var direct = max(dot(n, sun), 0.0);
        if mode > 5.5 {
            direct = smoothstep(-0.1, 0.1, dot(n, sun));
        }
        let sky = mix(vec3<f32>(0.20, 0.19, 0.18), vec3<f32>(0.39, 0.46, 0.55), n.z * 0.5 + 0.5);
        let light = c * (vec3<f32>(direct) + sky) + realism.glow;
        return vec4<f32>(filmic(light), in.color.a);
    }
    // Values: how light each colour is, as a grey.
    let grey = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return vec4<f32>(vec3<f32>(grey), in.color.a);
}

// Godot's Filmic tone mapping (its `tonemap_filmic`, white 1, exposure
// bias 2): bright light rolls off gently instead of clipping.
fn filmic_curve(x: vec3<f32>) -> vec3<f32> {
    let a = 0.22;
    let b = 0.30;
    let c = 0.10;
    let d = 0.20;
    let e = 0.01;
    let f = 0.30;
    return ((x * (a * x + c * b) + d * e) / (x * (a * x + b) + d * f)) - e / f;
}

fn filmic(color: vec3<f32>) -> vec3<f32> {
    let bias = 2.0;
    let white = filmic_curve(vec3<f32>(bias * 1.0));
    return clamp(filmic_curve(color * bias) / white, vec3<f32>(0.0), vec3<f32>(1.0));
}

// ------------------------------------------------------------ outlines ---

// An outline (decision 0201): the object's shell, each point pushed out
// along its smoothed direction by the width, in the world. Only its back
// is drawn, so it shows as a rim round the object.

struct OutlineIn {
    @location(0) position: vec3<f32>,
    @location(1) out: vec3<f32>,
};

struct OutlineOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn outline_vs(in: OutlineIn) -> OutlineOut {
    let out_world = (object.normal * vec4<f32>(in.out, 0.0)).xyz;
    let push = select(vec3<f32>(0.0), normalize(out_world), length(out_world) > 1e-8);
    let world = object_world(in.position) + push * object.outline.w;
    var out: OutlineOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    return out;
}

@fragment
fn outline_fs(in: OutlineOut, @builtin(front_facing) front: bool) -> SceneOut {
    // The shell's outside faces the camera only where it covers the
    // object: leave those out. A mirrored placement turns it inside out.
    let outside = front == (object.options.y > 0.0);
    if outside {
        discard;
    }
    var out: SceneOut;
    out.color = vec4<f32>(object.outline.rgb, 1.0);
    // Not a surface for cavity.
    out.facing = vec4<f32>(0.0);
    return out;
}

// How far apart, in pixels, the object's edges are on screen around
// `world`: its average edge length seen at that distance.
fn edge_spacing(world: vec3<f32>, clip: vec4<f32>) -> f32 {
    let size = max(frame.view.yz, vec2<f32>(1.0, 1.0));
    let to_world = normalize(world - frame.camera.xyz + vec3<f32>(0.0, 0.0, 1e-6));
    var across = cross(to_world, vec3<f32>(0.0, 0.0, 1.0));
    if length(across) < 1e-3 {
        across = cross(to_world, vec3<f32>(1.0, 0.0, 0.0));
    }
    let other = frame.view_proj * vec4<f32>(world + normalize(across) * object.options.z, 1.0);
    let here_px = clip.xy / max(clip.w, 1e-5) * size * 0.5;
    let there_px = other.xy / max(other.w, 1e-5) * size * 0.5;
    return length(there_px - here_px);
}

// A dense wireframe (edges only a few pixels apart, as on a heavy shape
// seen from afar) fades out, so the shape reads as a surface instead of a
// solid block of lines. Zoom in and the edges come back. A selected
// object keeps a faint tint of its colour: with many edges crossing each
// pixel, each one gets a share of it, so together they add up to about
// that much.
fn density_fade(world: vec3<f32>, clip: vec4<f32>) -> f32 {
    let spacing = edge_spacing(world, clip);
    let tint = object.options.w * clamp(spacing * 1.5, 0.0, 1.0);
    return max(smoothstep(1.5, 6.0, spacing), tint);
}

@vertex
fn mesh_line_vs(in: MeshIn) -> LineOut {
    let world = object_world(in.position);
    var out: LineOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    out.color = vec4<f32>(object.edge_color.rgb, object.edge_color.a * density_fade(world, out.clip));
    out.world = world;
    return out;
}

// An object's edges as strips `frame.lines.x` pixels wide (decision
// 0221): one instance an edge, six corners each (two triangles), its ends
// read from the mesh's vertices and edges as storage. The GPU's own lines
// are always one pixel, too thin on a high-density screen.
@group(2) @binding(0) var<storage, read> edge_vertices: array<f32>;
@group(2) @binding(1) var<storage, read> edge_ends: array<u32>;

// A `MeshVertex` is ten 4-byte numbers, its position first.
const VERTEX_FLOATS: u32 = 10u;

fn stored_position(vertex: u32) -> vec3<f32> {
    let at = vertex * VERTEX_FLOATS;
    return vec3<f32>(edge_vertices[at], edge_vertices[at + 1u], edge_vertices[at + 2u]);
}

@vertex
fn mesh_wide_line_vs(
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) edge: u32,
) -> LineOut {
    let a = object_world(stored_position(edge_ends[edge * 2u]));
    let b = object_world(stored_position(edge_ends[edge * 2u + 1u]));
    // Which end and which side each of the six corners is.
    var ends = array<f32, 6>(0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
    var sides = array<f32, 6>(-1.0, -1.0, 1.0, -1.0, 1.0, 1.0);
    let corner = index % 6u;
    var ca = frame.view_proj * vec4<f32>(a, 1.0);
    var cb = frame.view_proj * vec4<f32>(b, 1.0);
    // An end behind the camera is brought to just in front of it along
    // the edge, so the strip's direction on screen stays right.
    let near = 1e-4;
    if ca.w < near && cb.w >= near {
        ca = mix(ca, cb, (near - ca.w) / (cb.w - ca.w));
    } else if cb.w < near && ca.w >= near {
        cb = mix(cb, ca, (near - cb.w) / (ca.w - cb.w));
    }
    let size = max(frame.view.yz, vec2<f32>(1.0, 1.0));
    let pa = ca.xy / max(ca.w, near) * size * 0.5;
    let pb = cb.xy / max(cb.w, near) * size * 0.5;
    var along = pb - pa;
    if length(along) < 1e-5 {
        along = vec2<f32>(1.0, 0.0);
    }
    along = normalize(along);
    let across = vec2<f32>(-along.y, along.x);
    let at_b = ends[corner] > 0.5;
    let here = select(ca, cb, at_b);
    let world = select(a, b, at_b);
    let push = across * sides[corner] * frame.lines.x * 0.5;
    var out: LineOut;
    out.clip = vec4<f32>(here.xy + push * 2.0 / size * here.w, here.z, here.w);
    out.color = vec4<f32>(object.edge_color.rgb, object.edge_color.a * density_fade(world, here));
    out.world = world;
    // Both ends behind the camera: nothing to draw.
    if ca.w < near && cb.w < near {
        out.clip = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return out;
}

// Every point of an object as a small round dot: six corners per point,
// the point itself read once per dot.
@vertex
fn mesh_point_vs(@builtin(vertex_index) index: u32, @location(0) position: vec3<f32>) -> SpriteOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[index % 6u];
    let size = max(frame.view.yz, vec2<f32>(1.0, 1.0));
    let width = object.options.x;
    let world = object_world(position);
    var clip = frame.view_proj * vec4<f32>(world, 1.0);
    let fade = smoothstep(2.0, 6.0, edge_spacing(world, clip));
    let offset = corner * width * 0.5;
    clip = vec4<f32>(clip.xy + offset * 2.0 / size * clip.w, clip.z, clip.w);
    var out: SpriteOut;
    out.clip = clip;
    out.color = vec4<f32>(object.point_color.rgb, object.point_color.a * fade);
    out.corner = corner;
    out.round = 1.0;
    out.pixel = 2.0 / max(width, 1.0);
    return out;
}

// -------------------------------------------------------------- cavity ---

// Cavity (decision 0065): after the surfaces are drawn and before the
// grid, edges and overlay, one triangle over the whole view brightens
// ridges and darkens creases, from how the surfaces' facing changes
// across the screen. Where it turns away from the middle of a spot (a
// ridge, a box's outer corner) the light catches it; where it turns
// towards it (a crease, an inner corner) it is shaded.

@group(1) @binding(0) var facing_image: texture_2d<f32>;

struct CavityOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn cavity_vs(@builtin(vertex_index) index: u32) -> CavityOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    var out: CavityOut;
    out.clip = vec4<f32>(corners[index], 0.0, 1.0);
    return out;
}

// The facing at pixel `p`, or `fallback` off the image or off a surface
// (so a shape's outline is neither ridge nor crease).
fn facing_at(p: vec2<i32>, fallback: vec4<f32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(facing_image));
    if p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y {
        return fallback;
    }
    let f = textureLoad(facing_image, p, 0);
    return select(fallback, f, f.w > 0.99);
}

// How much the facing spreads apart around `p`, `r` pixels each way:
// above 0 on a ridge, below on a crease.
fn spread(p: vec2<i32>, here: vec4<f32>, r: i32) -> f32 {
    let left = facing_at(p - vec2<i32>(r, 0), here);
    let right = facing_at(p + vec2<i32>(r, 0), here);
    let above = facing_at(p - vec2<i32>(0, r), here);
    let below = facing_at(p + vec2<i32>(0, r), here);
    return (right.x - left.x) + (above.y - below.y);
}

@fragment
fn cavity_fs(in: CavityOut) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.clip.xy));
    let here = facing_at(p, vec4<f32>(0.0));
    if here.w < 0.99 {
        discard;
    }
    // A thin line and a softer one around it.
    let curve = 0.6 * spread(p, here, 1) + 0.4 * spread(p, here, 2);
    let ridge = clamp(curve * 0.5, 0.0, 1.0);
    let crease = clamp(-curve * 0.7, 0.0, 1.0);
    if ridge > crease {
        return vec4<f32>(1.0, 1.0, 1.0, ridge * 0.45);
    }
    return vec4<f32>(0.0, 0.0, 0.0, crease * 0.7);
}

// ------------------------------------------------- selection outline ---

// The selection's outline (decision 0220): after the surfaces, one
// triangle over the whole view (cavity_vs) colours the pixels just outside
// what a selected object covers on screen, so the selection shows without
// its edges. Only what can be seen is outlined: another object in front
// cuts the outline round it too.

// Whether pixel `p` shows a selected object's surface.
fn selected_at(p: vec2<i32>) -> bool {
    let size = vec2<i32>(textureDimensions(facing_image));
    if p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y {
        return false;
    }
    return textureLoad(facing_image, p, 0).w > 1.5;
}

@fragment
fn selection_fs(in: CavityOut) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.clip.xy));
    if selected_at(p) {
        discard;
    }
    // A selected pixel within the width, in eight directions: solid
    // within it, softened over its last pixel.
    let width = frame.selection.w;
    let reach = i32(ceil(width));
    var nearest = 1e9;
    for (var r = 1; r <= reach; r = r + 1) {
        for (var i = 0; i < 8; i = i + 1) {
            let a = f32(i) * 0.7853982;
            let d = vec2<i32>(round(vec2<f32>(cos(a), sin(a)) * f32(r)));
            if selected_at(p + d) {
                nearest = min(nearest, length(vec2<f32>(d)));
            }
        }
    }
    if nearest > width + 0.5 {
        discard;
    }
    let fade = clamp(width + 0.5 - nearest, 0.0, 1.0);
    return vec4<f32>(frame.selection.rgb, fade);
}
