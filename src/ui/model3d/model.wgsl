// Shorashim: PBR-ish model pass (offscreen, 4x MSAA + depth) and blit.

struct U {
    // yaw, pitch, aspect, output_is_srgb (1/0)
    view: vec4<f32>,
    // model centre xyz, 1/radius
    fit: vec4<f32>,
    // base colour factor
    base: vec4<f32>,
    // metallic factor, roughness factor, has_normal_map, warm (theme tint 0..1)
    mat: vec4<f32>,
    // zoom, hover glow (0..1), 0, 0
    fx: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var t_base: texture_2d<f32>;
@group(0) @binding(3) var t_mr: texture_2d<f32>;
@group(0) @binding(4) var t_nrm: texture_2d<f32>;

const CAM_D: f32 = 4.2;
const FOCAL: f32 = 3.6;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) vpos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

fn rot(p: vec3<f32>) -> vec3<f32> {
    let cy = cos(u.view.x); let sy = sin(u.view.x);
    let cp = cos(u.view.y); let sp = sin(u.view.y);
    let a = vec3<f32>(cy * p.x + sy * p.z, p.y, -sy * p.x + cy * p.z);
    return vec3<f32>(a.x, cp * a.y - sp * a.z, sp * a.y + cp * a.z);
}

@vertex
fn vs_model(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @location(2) uv: vec2<f32>) -> VOut {
    var o: VOut;
    let p = rot((pos - u.fit.xyz) * u.fit.w);
    let v = p - vec3<f32>(0.0, 0.0, CAM_D);
    let aspect = u.view.z;
    let fy = FOCAL * u.fx.x * min(aspect, 1.0);
    let fx = fy / aspect;
    let near = 0.5; let far = 10.0;
    o.clip = vec4<f32>(v.x * fx, v.y * fy, v.z * far / (near - far) + near * far / (near - far), -v.z);
    o.vpos = v;
    o.nrm = rot(nrm);
    o.uv = uv;
    return o;
}

const PI: f32 = 3.14159265;

// Procedural studio environment: a big warm softbox above, two tall strip
// lights at the sides (warm left, cool right), a dim front fill behind the
// camera, a dark horizon band and a warm wooden floor bounce. Metal reads by
// what it reflects, so the contrast here is what makes silver look silver.
fn strip(d: vec3<f32>, az: vec2<f32>, sharp: f32) -> f32 {
    let h = normalize(vec2<f32>(d.x, d.z) + vec2<f32>(1e-5, 0.0));
    return pow(max(dot(h, normalize(az)), 0.0), sharp) * smoothstep(-0.35, 0.05, d.y) * (1.0 - smoothstep(0.55, 0.85, d.y));
}

fn env(d: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.035, 0.03, 0.028);
    c += vec3<f32>(1.25, 1.15, 1.0) * smoothstep(0.45, 0.95, d.y);              // top softbox
    c += vec3<f32>(0.16, 0.10, 0.06) * smoothstep(0.0, -0.6, d.y);               // floor bounce
    c += vec3<f32>(3.2, 2.7, 2.0) * strip(d, vec2<f32>(-0.95, 0.30), 90.0);      // left strip
    c += vec3<f32>(1.8, 2.0, 2.4) * strip(d, vec2<f32>(0.90, -0.35), 70.0);      // right rim strip
    c += vec3<f32>(0.30, 0.27, 0.24) * strip(d, vec2<f32>(0.0, 1.0), 4.0);       // front fill
    let key = normalize(vec3<f32>(-0.5, 0.7, 0.6));
    c += vec3<f32>(6.0, 5.4, 4.6) * pow(max(dot(d, key), 0.0), 60.0);
    return c;
}

fn env_brdf(spec: vec3<f32>, rough: f32, nov: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = rough * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * nov)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return spec * ab.x + ab.y;
}

fn ggx(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
    let a = max(rough * rough, 0.002);
    let a2 = a * a;
    let noh = max(dot(n, h), 0.0);
    let nol = max(dot(n, l), 0.0);
    let nov = max(dot(n, v), 1e-4);
    let d = a2 / (PI * pow(noh * noh * (a2 - 1.0) + 1.0, 2.0));
    let k = a * 0.5;
    let g = (nol / (nol * (1.0 - k) + k)) * (nov / (nov * (1.0 - k) + k));
    let f = f0 + (1.0 - f0) * pow(1.0 - max(dot(v, h), 0.0), 5.0);
    return d * g * f / max(4.0 * nol * nov, 1e-4);
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_model(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let base_s = textureSample(t_base, samp, i.uv);
    let mr = textureSample(t_mr, samp, i.uv);
    let tn_s = textureSample(t_nrm, samp, i.uv).xyz;
    let dp1 = dpdx(i.vpos);
    let dp2 = dpdy(i.vpos);
    let duv1 = dpdx(i.uv);
    let duv2 = dpdy(i.uv);

    var n = normalize(i.nrm);
    if (!front) { n = -n; }
    if (u.mat.z > 0.5) {
        // Cotangent frame from screen derivatives (no tangent attribute).
        let dp2perp = cross(dp2, n);
        let dp1perp = cross(n, dp1);
        let t = dp2perp * duv1.x + dp1perp * duv2.x;
        let b = dp2perp * duv1.y + dp1perp * duv2.y;
        let inv = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-20));
        let tn = tn_s * 2.0 - 1.0;
        // glTF normal maps are +Y up with V pointing down the image.
        let pn = normalize(t * inv * tn.x - b * inv * tn.y + n * tn.z);
        if (all(pn == pn)) { n = pn; }
    }

    let albedo = base_s.rgb * u.base.rgb;
    let metal = clamp(mr.b * u.mat.x, 0.0, 1.0);
    let rough = clamp(mr.g * u.mat.y, 0.04, 1.0);
    let v = normalize(-i.vpos);
    let nov = max(dot(n, v), 1e-4);
    let f0 = mix(vec3<f32>(0.04), albedo, metal);
    let diff = albedo * (1.0 - metal);

    // Ambient: hemisphere diffuse + blurred env specular.
    let hemi = mix(vec3<f32>(0.12, 0.09, 0.07), vec3<f32>(0.45, 0.42, 0.38), n.y * 0.5 + 0.5);
    let r = reflect(-v, n);
    let sharp = env(r);
    let blurred = mix(env(normalize(mix(r, n, 0.6))), hemi * 1.6, 0.5);
    let spec_env = mix(sharp, blurred, clamp(rough * rough * 1.6, 0.0, 1.0));
    var col = diff * hemi + spec_env * env_brdf(f0, rough, nov);

    // Two lights: warm key, cool rim.
    let keys = array<vec3<f32>, 2>(normalize(vec3<f32>(-0.5, 0.7, 0.6)), normalize(vec3<f32>(0.8, 0.25, -0.4)));
    let kcol = array<vec3<f32>, 2>(vec3<f32>(2.6, 2.35, 2.0), vec3<f32>(0.7, 0.8, 1.0));
    for (var k = 0; k < 2; k++) {
        let l = keys[k];
        let nol = max(dot(n, l), 0.0);
        col += (diff / PI * 2.2 + ggx(n, v, l, rough, f0)) * kcol[k] * nol;
    }
    // Warm theme tint (candle light), small.
    col *= mix(vec3<f32>(1.0), vec3<f32>(1.06, 1.0, 0.9), u.mat.w);
    // Hover: a warm candle-gold rim and a touch more light.
    let rim = pow(1.0 - nov, 3.0);
    col = col * (1.0 + 0.12 * u.fx.y) + vec3<f32>(1.0, 0.72, 0.32) * rim * 0.9 * u.fx.y;
    // Per-model exposure cut on the dark themes (fx.z, 0 = none).
    var out = aces(col * 1.1 * (1.0 - u.fx.z));
    if (u.view.w < 0.5) { out = pow(out, vec3<f32>(1.0 / 2.2)); }
    return vec4<f32>(out, 1.0);
}

// ---- blit: offscreen (premultiplied) over iced's frame + contact shadow ----

@group(0) @binding(0) var b_samp: sampler;
@group(0) @binding(1) var b_tex: texture_2d<f32>;
@group(0) @binding(2) var<uniform> b_u: vec4<f32>; // shadow centre y (0..1), width, height, strength

struct BOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) i: u32) -> BOut {
    var o: BOut;
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    o.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs_blit(i: BOut) -> @location(0) vec4<f32> {
    let m = textureSample(b_tex, b_samp, i.uv);
    let q = (i.uv - vec2<f32>(0.5, b_u.x)) / vec2<f32>(b_u.y, b_u.z);
    let s = b_u.w * (1.0 - smoothstep(0.0, 1.0, length(q)));
    // premultiplied: model over shadow
    return vec4<f32>(m.rgb, m.a + s * (1.0 - m.a));
}
