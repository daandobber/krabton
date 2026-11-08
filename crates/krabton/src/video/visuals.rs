//! The Visuals: pictures Krabton makes itself, drawn by the graphics card
//! before the Look's effects (so every effect goes on them too). What they
//! hear and see of the music comes in as a small picture, the *scope*: the
//! master's wave (triggered, as an oscilloscope holds it still), its
//! spectrum (128 bands from 40 Hz to 16 kHz) and how lately and how hard
//! every note was played. How they move is the Look's: the Visuals effect
//! (its scene, blend, colours and speed) and Burst, Warp, Density, Rush,
//! Glow, Orbit and Fracture, each moved by links like any effect.

/// The width of the scope (its rows: the wave, the spectrum, the notes).
pub const SCOPE_W: usize = 128;
/// Samples kept of the master, and the size of the spectrum's FFT.
const TAIL: usize = 4096;
const FFT: usize = 2048;

/// What the Visuals are shown of the music this frame (each 0..255; the
/// wave's middle is 128).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scope {
    pub wave: [u8; SCOPE_W],
    pub spec: [u8; SCOPE_W],
    pub notes: [u8; SCOPE_W],
}

impl Default for Scope {
    fn default() -> Self {
        Self { wave: [128; SCOPE_W], spec: [0; SCOPE_W], notes: [0; SCOPE_W] }
    }
}

impl Scope {
    /// As the scope picture: 128 by 4, RGBA, a row each (the fourth empty).
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = vec![0u8; SCOPE_W * 4 * 4];
        for (row, vals) in [&self.wave, &self.spec, &self.notes].into_iter().enumerate() {
            for (x, v) in vals.iter().enumerate() {
                let i = (row * SCOPE_W + x) * 4;
                out[i..i + 4].copy_from_slice(&[*v, *v, *v, 255]);
            }
        }
        out
    }
}

/// The Visuals' ears, between frames.
pub struct Ears {
    tail: Vec<f32>,
    spec: [f32; SCOPE_W],
    notes: [f32; SCOPE_W],
    /// How loud the wave was lately (it is drawn as big whatever the level).
    gain: f32,
    /// The Visuals' own time: it runs as fast as their Speed and Rush say (seconds, wrapped at an hour).
    pub time: f64,
    /// How long the last frame was (seconds).
    pub dt: f32,
}

impl Default for Ears {
    fn default() -> Self {
        Self { tail: Vec::with_capacity(TAIL * 2), spec: [0.0; SCOPE_W], notes: [0.0; SCOPE_W], gain: 0.0, time: 0.0, dt: 1.0 / 60.0 }
    }
}

/// How much of `v` is left after `dt` seconds of falling with half-life `half`.
fn fall(dt: f32, half: f32) -> f32 {
    0.5f32.powf(dt / half.max(1e-4))
}

impl Ears {
    /// One frame: the master's samples since the last, the notes that
    /// started (a bit each) and how hard, `dt` seconds on, the Visuals
    /// going `speed` times as fast as they do by themselves.
    pub fn frame(&mut self, samples: &[f32], struck: u128, velocity: f32, dt: f32, sample_rate: f32, speed: f32) {
        self.tail.extend_from_slice(samples);
        if self.tail.len() > TAIL {
            self.tail.drain(..self.tail.len() - TAIL);
        }
        // The spectrum: up at once, falling away.
        let now = spectrum(&self.tail, sample_rate);
        let k = fall(dt, 0.12);
        for (s, n) in self.spec.iter_mut().zip(now) {
            *s = n.max(*s * k);
        }
        // The notes: as hard as they start, falling away.
        let k = fall(dt, 0.35);
        for (n, v) in self.notes.iter_mut().enumerate() {
            *v *= k;
            if struck & (1u128 << n) != 0 {
                *v = v.max(velocity.clamp(0.2, 1.0));
            }
        }
        let peak = self.tail.iter().rev().take(512).fold(0.0f32, |m, v| m.max(v.abs()));
        self.gain = if peak > self.gain { peak } else { self.gain * fall(dt, 1.5) + peak * (1.0 - fall(dt, 1.5)) };
        self.time = (self.time + dt as f64 * speed.max(0.0) as f64).rem_euclid(3600.0);
        self.dt = dt;
    }

    /// What the Visuals are shown now.
    pub fn scope(&self) -> Scope {
        let mut s = Scope::default();
        // The wave: 256 samples from where it last rises through the middle (it stands still).
        let n = self.tail.len();
        if n >= 512 {
            let from = n - 512;
            let start = (from..from + 256).find(|&i| self.tail[i] <= 0.0 && self.tail[i + 1] > 0.0).unwrap_or(from);
            let g = 0.9 / self.gain.max(0.02);
            for (x, w) in s.wave.iter_mut().enumerate() {
                let v = self.tail[start + x * 2] * g;
                *w = (128.0 + v.clamp(-1.0, 1.0) * 127.0).round() as u8;
            }
        }
        for (o, v) in s.spec.iter_mut().zip(self.spec) {
            *o = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        for (o, v) in s.notes.iter_mut().zip(self.notes) {
            *o = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        s
    }
}

/// An FFT in place (its length a power of two).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let a = -std::f32::consts::TAU / len as f32;
        let (wr, wi) = (a.cos(), a.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (i, j) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[j] * cr - im[j] * ci, re[j] * ci + im[j] * cr);
                re[j] = re[i] - tr;
                im[j] = im[i] - ti;
                re[i] += tr;
                im[i] += ti;
                (cr, ci) = (cr * wr - ci * wi, cr * wi + ci * wr);
            }
        }
        len <<= 1;
    }
}

/// How loud the last `FFT` samples are in 128 bands from 40 Hz to 16 kHz
/// (0..1: from 72 dB down to 6 dB down, tilted up 3 dB an octave so the top shows).
pub fn spectrum(tail: &[f32], sample_rate: f32) -> [f32; SCOPE_W] {
    let mut out = [0.0; SCOPE_W];
    if tail.len() < FFT || sample_rate <= 0.0 {
        return out;
    }
    let x = &tail[tail.len() - FFT..];
    let mut re: Vec<f32> = x.iter().enumerate().map(|(i, v)| v * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FFT as f32).cos())).collect();
    let mut im = vec![0.0; FFT];
    fft(&mut re, &mut im);
    let mag = |k: usize| (re[k] * re[k] + im[k] * im[k]).sqrt() * 4.0 / FFT as f32;
    let hz = |b: f32| 40.0 * (16000.0f32 / 40.0).powf(b / (SCOPE_W - 1) as f32);
    let bin = |f: f32| (f * FFT as f32 / sample_rate).clamp(1.0, (FFT / 2 - 1) as f32);
    for (b, o) in out.iter_mut().enumerate() {
        let (lo, hi) = (bin(hz(b as f32 - 0.5)), bin(hz(b as f32 + 0.5)));
        let m = if hi - lo < 1.0 {
            // Fewer bins than bands down low: between the two nearest.
            let c = bin(hz(b as f32));
            let (i, f) = (c.floor() as usize, c.fract());
            mag(i) * (1.0 - f) + mag((i + 1).min(FFT / 2 - 1)) * f
        } else {
            (lo.ceil() as usize..=hi.floor() as usize).map(mag).fold(0.0, f32::max)
        };
        let db = 20.0 * m.max(1e-9).log10() + 3.0 * (hz(b as f32) / 1000.0).log2();
        *o = ((db + 72.0) / 66.0).clamp(0.0, 1.0);
    }
    out
}

/// The shader that draws the Visuals into a picture of their own (the
/// effects' `A_*` and `P_*` come first, made by `stage::defines`).
pub const SHADER: &str = r#"
uniform vec2 u_stage;
uniform vec4 u_fx[FX_VEC4];
uniform vec4 u_p[FX_N];
uniform float u_time;
uniform float u_seed;
uniform vec3 u_vcol;
uniform sampler2D u_scope;
uniform sampler2D u_milk;
uniform vec4 u_m[MILK_VEC4];
I vec2 v_uv;

#define TAU 6.2831853

float h11(float n) { return fract(sin(n * 12.9898) * 43758.5453); }
float h21(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
vec2 h22(vec2 p) { float n = h21(p); return vec2(n, h21(p + n + 17.17)); }
float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(h21(i), h21(i + vec2(1.0, 0.0)), u.x), mix(h21(i + vec2(0.0, 1.0)), h21(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p) {
    float s = 0.0;
    float a = 0.5;
    for (int i = 0; i < 4; i++) {
        s += a * vnoise(p);
        p = p * 2.03 + vec2(1.7, 9.2);
        a *= 0.5;
    }
    return s;
}
vec2 rot(vec2 p, float a) { return vec2(cos(a) * p.x - sin(a) * p.y, sin(a) * p.x + cos(a) * p.y); }

// The music: the wave at x (0..1, -1..1), the spectrum at x (0..1, low to high), how lately note n was played.
float wave(float x) { return texture2D(u_scope, vec2(clamp(x, 0.0, 1.0), 0.125)).r * 2.0 - 1.0; }
float spec(float x) { return texture2D(u_scope, vec2(clamp(x, 0.0, 1.0) * (127.0 / 128.0) + 0.5 / 128.0, 0.375)).r; }
float note(float n) { return texture2D(u_scope, vec2((clamp(n, 0.0, 127.0) + 0.5) / 128.0, 0.625)).r; }

// Its colours: along `t` (0..1).
vec3 palette(float t) {
    float pal = P_VISUALS.z;
    if (pal < 0.5) { return u_vcol; }
    if (pal < 1.5) { return mix(vec3(0.15, 0.55, 1.0), vec3(0.85, 0.97, 1.0), t); }
    if (pal < 2.5) { return mix(vec3(1.0, 0.22, 0.04), vec3(1.0, 0.86, 0.35), t); }
    if (pal < 3.5) { return mix(vec3(1.0, 0.1, 0.72), vec3(0.1, 0.9, 1.0), t); }
    if (pal < 4.5) { return mix(vec3(0.3, 1.0, 0.1), vec3(0.95, 1.0, 0.25), t); }
    return 0.5 + 0.5 * cos(TAU * (t + vec3(0.0, 0.33, 0.67)));
}

// A line `d` away (in heights of the stage): sharp, and glowing as Glow says.
float lit(float d, float w) {
    float px = 1.0 / u_stage.y;
    float core = smoothstep(w * px + px, w * px - px * 0.5, abs(d));
    float reach = px * (2.0 + 18.0 * P_GLOW.x);
    return core + exp(-abs(d) / reach) * (0.05 + 0.45 * A_GLOW);
}

float T;
float burst;
float warp;
float dens;
float orbit;
float aspect;

vec3 ridges(vec2 p) {
    vec3 col = vec3(0.0);
    float n = floor(28.0 * dens);
    for (int i = 0; i < 112; i++) {
        float fi = float(i);
        if (fi >= n) { break; }
        float z = fi / max(n - 1.0, 1.0);
        float base = mix(0.3, -0.42, z);
        if (p.y > base + 0.6 || p.y < base - 0.08) { continue; }
        float xs = p.x * mix(1.6, 0.75, z);
        float env = exp(-xs * xs * 3.0);
        float ground = vnoise(vec2(xs * 3.0 * P_WARP.x, z * 6.0 - T * 0.8)) * 0.5 + vnoise(vec2(xs * 7.0, z * 13.0 - T * 1.6)) * 0.25;
        float s = spec(abs(xs) * 0.8);
        float h = env * (ground * (0.12 + 0.3 * warp) + s * 0.16 + burst * 0.3 * h11(fi + floor(T * 4.0)));
        float y = base + h * (0.4 + z);
        if (p.y < y) { col = vec3(0.0); }
        col += palette(z) * lit(p.y - y, 1.0 + z);
    }
    return col;
}

float lattice_d(vec3 q, float cell, float r) {
    q = mod(q, cell) - 0.5 * cell;
    return min(min(length(q.xy), length(q.yz)), length(q.xz)) - r;
}

vec3 lattice(vec2 p) {
    vec3 ro = vec3(0.0, 0.0, -T * 2.5);
    vec3 rd = normalize(vec3(p, 1.1 - burst * 0.6));
    rd.xz = rot(rd.xz, orbit);
    rd.xy = rot(rd.xy, T * 0.07 + orbit * 0.5);
    float cell = 2.2 / sqrt(dens);
    float r = 0.025 + burst * 0.04;
    float t = 0.0;
    vec3 glow = vec3(0.0);
    float hit = 0.0;
    for (int i = 0; i < 56; i++) {
        vec3 q = ro + rd * t;
        q.xy = rot(q.xy, q.z * warp * 0.25);
        float d = lattice_d(q, cell, r);
        float band = spec(fract(abs(q.z) * 0.05));
        glow += palette(fract(q.z * 0.03)) * exp(-d * 18.0) * 0.012 * (0.4 + band * 1.6);
        if (d < 0.002) { hit = 1.0; break; }
        t += d;
        if (t > 30.0) { break; }
    }
    float fog = exp(-t * 0.09);
    return palette(fract((ro.z + rd.z * t) * 0.03)) * hit * fog * 0.9 + glow * (1.0 + 2.0 * A_GLOW);
}

vec3 swarm(vec2 p) {
    vec3 col = vec3(0.0);
    float a = orbit + T * 0.15;
    float m = floor(26.0 * dens);
    for (int k = 0; k < 9; k++) {
        float fk = float(k);
        float r = 0.14 + 0.045 * fk + burst * 0.5 * h11(fk + 3.0) * (0.3 + fk * 0.1);
        r *= 1.0 + warp * 0.25 * sin(atan(p.y, p.x) * 3.0 + T + fk);
        float rr = dot(p, p);
        if (rr > r * r) { continue; }
        float zz = sqrt(r * r - rr);
        for (int side = 0; side < 2; side++) {
            float z = side == 0 ? -zz : zz;
            vec3 q = vec3(p, z);
            q.xz = rot(q.xz, a * (1.0 + fk * 0.13));
            q.yz = rot(q.yz, 0.4 + fk * 0.3);
            vec2 sph = vec2(atan(q.y, q.x) / TAU + 0.5, acos(clamp(q.z / r, -1.0, 1.0)) / 3.14159);
            vec2 g = sph * vec2(m * 2.0, m);
            vec2 id = floor(g);
            vec2 at = fract(g) - h22(id + fk * 31.0);
            float d = length(at * vec2(TAU * r / (m * 2.0), 3.14159 * r / m) * vec2(sin(sph.y * 3.14159) + 0.05, 1.0));
            float size = (1.2 + 2.5 * h21(id + 7.0)) / u_stage.y;
            float front = side == 0 ? 1.0 : 0.35;
            float flick = 0.5 + 0.5 * spec(h21(id));
            col += palette(fk / 8.0) * front * flick * (smoothstep(size, 0.0, d) + exp(-d / (size * (2.0 + 6.0 * A_GLOW))) * 0.2);
        }
    }
    return col;
}

vec3 data(vec2 uv) {
    float cols = floor(40.0 * dens);
    float c = floor(uv.x * cols);
    float dir = h11(c) < 0.5 ? -1.0 : 1.0;
    float y = uv.y + T * (0.1 + h11(c + 9.0) * 0.9) * dir;
    float rows = 30.0 + floor(h11(c + 4.0) * 3.0) * 20.0;
    float b = floor(y * rows);
    float bit = step(0.55 - spec(c / cols) * 0.45, h21(vec2(c, b)));
    vec2 inside = vec2(fract(uv.x * cols), fract(y * rows));
    float box = step(0.08, inside.x) * step(inside.x, 0.92) * step(0.12, inside.y) * step(inside.y, 0.88);
    vec3 col = palette(c / cols) * bit * box * 0.55;
    // The spectrum's bars, up from the bottom.
    float s = spec(uv.x);
    float bar = step(1.0 - uv.y, s * 0.6) * step(0.15, fract(uv.x * 128.0));
    col = max(col, palette(uv.x) * bar);
    // A barcode across the middle, flickering with the top.
    float stripe = step(0.5, h21(vec2(floor(uv.x * (90.0 + 120.0 * spec(0.9))), floor(T * 12.0))));
    float band = step(abs(uv.y - 0.5 + sin(T * 0.3) * 0.2), 0.02 + 0.06 * spec(0.8));
    col = max(col, vec3(stripe * band));
    // A scan line.
    col += palette(0.5) * lit(uv.y - fract(T * 0.25), 1.0) * 0.8;
    // Burst: blocks turned over.
    float flip = step(h21(floor(uv * vec2(8.0, 6.0)) + floor(T * 6.0)), burst * 0.7);
    return mix(col, 1.0 - col, flip);
}

vec3 tunnel(vec2 p) {
    vec2 q = rot(p, orbit + T * 0.1);
    float r = max(length(q), 0.001);
    float a = atan(q.y, q.x) / TAU + 0.5;
    float z = 0.3 / r + T * 1.6 - burst * 0.8;
    a += warp * 0.08 * sin(z * 1.5);
    float rings = 2.0 * dens;
    float f = fract(z * rings);
    // A line's width in depth here (a pixel and a half on the screen).
    float w = rings * 0.3 / (r * r) * 1.5 / u_stage.y;
    float ring = smoothstep(w * 1.5, w * 0.3, min(f, 1.0 - f));
    float segs = 32.0;
    float sa = fract(a) * segs;
    float seg = floor(sa);
    float sw = segs / (TAU * r) * 1.5 / u_stage.y;
    float spoke = smoothstep(sw * 1.5, sw * 0.3, min(fract(sa), 1.0 - fract(sa))) * 0.35;
    float level = spec(abs(seg / segs * 2.0 - 1.0));
    float on = step(h21(vec2(seg, floor(z * rings))), 0.12 + level * 1.2);
    float tile = on * (1.0 - ring) * 0.3;
    vec3 col = palette(fract(z * 0.05)) * (ring + spoke + tile);
    return col * smoothstep(0.02, 0.3, r) * (1.0 + A_GLOW);
}

vec3 scope(vec2 p) {
    vec3 col = vec3(0.0);
    float amp = 0.32 * (1.0 + burst * 1.5);
    float round_ = smoothstep(0.2, 0.8, A_ORBIT);
    int copies = int(1.0 + floor(dens));
    for (int c = 0; c < 4; c++) {
        if (c >= copies) { break; }
        float fc = float(c);
        // Across the screen.
        float x = p.x / aspect + 0.5;
        float off = (fc - (float(copies) - 1.0) * 0.5) * 0.18;
        float bend = warp * 0.15 * sin(x * 9.0 + T * 2.0 + fc);
        float y = wave(x) * amp * (1.0 - fc * 0.2) + off + bend;
        float dx = 1.0 / 128.0;
        float slope = (wave(x + dx) - wave(x - dx)) * amp / (2.0 * dx * aspect);
        float lin = lit((p.y - y) / sqrt(1.0 + slope * slope), 1.5);
        // Round.
        vec2 q = rot(p, A_ORBIT * P_ORBIT.x * TAU + T * 0.2);
        float ang = atan(q.y, q.x) / TAU + 0.5;
        float rr = 0.24 + fc * 0.07 + wave(ang) * 0.1 * (1.0 + burst * 1.5) + bend * 0.3;
        float cir = lit(length(q) - rr, 1.5);
        col += palette(fc / 3.0) * mix(lin, cir, round_);
    }
    // The spectrum underneath, mirrored about the middle.
    float sx = abs(p.x / aspect) * 2.0;
    float s = spec(sx);
    col += palette(sx) * step(p.y, -0.5 + s * 0.25) * step(0.2, fract(sx * 64.0)) * 0.6;
    return col;
}

vec3 contours(vec2 p) {
    vec2 q = rot(p, orbit) * 1.6;
    vec2 e = vec2(1.5 / u_stage.y, 0.0);
    float lv = 10.0 * dens;
    float h0 = 0.0;
    float hx = 0.0;
    float hy = 0.0;
    for (int k = 0; k < 3; k++) {
        vec2 s = q + (k == 1 ? e.xy : (k == 2 ? e.yx : vec2(0.0)));
        vec2 w = s + warp * 0.7 * vec2(fbm(s * 1.3 * P_WARP.x + T * 0.1), fbm(s * 1.3 * P_WARP.x + 5.2 - T * 0.1));
        float h = fbm(w * 1.2 + vec2(0.0, T * 0.12));
        float l = length(s);
        h += burst * 0.12 * sin(l * 22.0 - T * 9.0) * exp(-l * 1.5);
        if (k == 0) { h0 = h; } else if (k == 1) { hx = h; } else { hy = h; }
    }
    float g = length(vec2(hx - h0, hy - h0)) / e.x;
    float f = fract(h0 * lv);
    float d = min(f, 1.0 - f) / max(g * lv, 1e-4);
    float major = step(0.5, fract(floor(h0 * lv) * 0.2 + 0.1));
    return palette(fract(h0 * 1.5)) * (lit(d, major > 0.5 ? 1.6 : 0.8) + 0.04);
}

vec3 shards(vec2 p) {
    vec2 q = rot(p, orbit) * (4.0 * dens);
    q += warp * 0.6 * sin(q.yx * 1.3 + T);
    vec2 i = floor(q);
    float d1 = 8.0;
    float d2 = 8.0;
    vec2 best = vec2(0.0);
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 c = i + vec2(float(x), float(y));
            vec2 o = h22(c);
            o = 0.5 + 0.42 * sin(T * 0.6 + TAU * o);
            vec2 pt = c + o;
            pt += normalize(pt + 0.001) * burst * 1.2 * h21(c + 3.3);
            float d = length(pt - q);
            if (d < d1) { d2 = d1; d1 = d; best = c; } else if (d < d2) { d2 = d; }
        }
    }
    float edge = (d2 - d1) / (4.0 * dens) ;
    float pitch = 36.0 + floor(h21(best) * 48.0);
    float on = note(pitch);
    float fill = on + spec(h21(best + 1.0)) * 0.25;
    vec3 col = palette(h21(best + 2.0)) * fill * 0.7;
    return col + palette(0.8) * lit(edge, 0.8);
}

vec3 notegrid(vec2 uv) {
    vec2 q = uv - 0.5;
    q = rot(q * vec2(aspect, 1.0), orbit) / vec2(aspect, 1.0);
    q /= 1.0 + burst * 0.3;
    q += 0.5;
    vec2 g = vec2(q.x * 12.0, (1.0 - q.y) * 8.0 + 1.0);
    vec2 id = floor(g);
    if (q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0) { return vec3(0.0); }
    vec2 f = fract(g) - 0.5;
    float v = note(id.y * 12.0 + id.x);
    float box = max(abs(f.x), abs(f.y));
    float cell = smoothstep(0.42, 0.40, box);
    float outline = smoothstep(0.03, 0.0, abs(box - 0.43)) * 0.18;
    float halo = exp(-box * (8.0 - 5.0 * A_GLOW)) * v * 0.6;
    vec3 col = palette(id.x / 11.0);
    return col * (cell * v + outline + halo) + col * spec(q.x) * 0.06;
}

vec3 strings(vec2 p) {
    vec3 col = vec3(0.0);
    float x = p.x / aspect + 0.5;
    float ends = sin(3.14159 * clamp(x, 0.0, 1.0));
    for (int i = 24; i < 108; i++) {
        float n = float(i);
        float y0 = mix(-0.42, 0.42, (n - 24.0) / 84.0);
        if (abs(p.y - y0) > 0.2) { continue; }
        float v = note(n);
        float freq = pow(2.0, (n - 60.0) / 24.0) * (2.0 + dens * 2.0);
        float shake = v * (0.07 + burst * 0.1) * ends * sin(x * freq * TAU + T * (6.0 + n * 0.05)) * (1.0 + warp * sin(x * 13.0 + T));
        float idle = mod(n, 12.0) < 0.5 ? 0.16 : 0.05;
        col += palette((n - 24.0) / 84.0) * lit(p.y - y0 - shake, 1.0 + v) * max(v, idle);
    }
    return col;
}

// Milk's feedback as it is shown: an echo of it, gamma, brighter, darker, solarized, negative, its colours turned.
vec3 milk(vec2 uv) {
    vec3 c = texture2D(u_milk, vec2(uv.x, 1.0 - uv.y)).rgb;
    if (ECHO_ALPHA > 0.001) {
        vec2 e = (uv - 0.5) / max(ECHO_ZOOM, 0.05) + 0.5;
        if (mod(ECHO_ORIENT, 2.0) > 0.5) { e.x = 1.0 - e.x; }
        if (ECHO_ORIENT > 1.5) { e.y = 1.0 - e.y; }
        vec3 ec = texture2D(u_milk, vec2(fract(e.x), 1.0 - fract(e.y))).rgb;
        c = mix(c, max(c, ec), ECHO_ALPHA);
    }
    c *= GAMMA;
    c = mix(c, sqrt(clamp(c, 0.0, 1.0)), BRIGHTEN);
    c = mix(c, c * c, DARKEN);
    c = mix(c, clamp(c, 0.0, 1.0) * (1.0 - clamp(c, 0.0, 1.0)) * 4.0, SOLARIZE);
    c = mix(c, 1.0 - clamp(c, 0.0, 1.0), INVERT);
    if (HUE > 0.001) {
        vec3 k = vec3(0.57735);
        float a = HUE * TAU;
        c = c * cos(a) + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - cos(a));
    }
    return c;
}

void main() {
    aspect = u_stage.x / u_stage.y;
    T = u_time;
    burst = A_BURST * P_BURST.x;
    warp = A_WARP;
    dens = 1.0 + A_DENSITY * (P_DENSITY.x - 1.0);
    orbit = A_ORBIT * P_ORBIT.x * TAU;
    vec2 uv = v_uv;
    // Fracture: blocks pushed about.
    if (A_FRACTURE > 0.001) {
        vec2 b = floor(uv * vec2(P_FRACTURE.x, P_FRACTURE.x / aspect));
        float r = h21(b + floor(T * 7.0) + u_seed);
        if (r < A_FRACTURE * 0.65) {
            uv += (h22(b + 3.1) - 0.5) * 0.3 * A_FRACTURE;
        }
    }
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);
    float scene = P_VISUALS.x;
    vec3 c;
    if (scene < 0.5) { c = ridges(p); }
    else if (scene < 1.5) { c = lattice(p); }
    else if (scene < 2.5) { c = swarm(p); }
    else if (scene < 3.5) { c = data(uv); }
    else if (scene < 4.5) { c = tunnel(p); }
    else if (scene < 5.5) { c = scope(p); }
    else if (scene < 6.5) { c = contours(p); }
    else if (scene < 7.5) { c = shards(p); }
    else if (scene < 8.5) { c = notegrid(uv); }
    else if (scene < 9.5) { c = strings(p); }
    else { c = milk(uv); }
    c *= 1.0 + A_GLOW * 0.5;
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
"#;

/// The Milk values' names in the shaders: `ZOOM` for `u_m[0].x`, ...
pub fn milk_defines() -> String {
    use krabton_engine::milk::{MilkParam, MILK_N};
    let mut s = format!("#define MILK_VEC4 {}\n", MILK_N.div_ceil(4));
    for m in MilkParam::ALL {
        let i = m.index();
        s.push_str(&format!("#define {} u_m[{}].{}\n", m.glsl(), i / 4, ["x", "y", "z", "w"][i % 4]));
    }
    s
}

/// Milk's values packed four to a vec4, for `u_m`.
pub fn milk_packed(v: &[f32; krabton_engine::milk::MILK_N]) -> Vec<f32> {
    let mut out = v.to_vec();
    out.resize(v.len().div_ceil(4) * 4, 0.0);
    out
}

/// Milk's feedback, into a picture of its own: the last frame pulled
/// through the field of motion and faded, the wave, the shape (which can
/// hold the last frame), motion dots and borders over it. `u_k` is the
/// frames of 60 a second this frame is (so it moves alike at any rate).
pub const MILK_SHADER: &str = r#"
uniform sampler2D u_prev;
uniform sampler2D u_scope;
uniform vec4 u_m[MILK_VEC4];
uniform vec2 u_stage;
uniform float u_time;
uniform float u_k;
I vec2 v_uv;

#define TAU 6.2831853

float wave(float x) { return texture2D(u_scope, vec2(fract(x), 0.125)).r * 2.0 - 1.0; }
float spec(float x) { return texture2D(u_scope, vec2(clamp(x, 0.0, 1.0) * (127.0 / 128.0) + 0.5 / 128.0, 0.375)).r; }
vec3 hsv(float h, float s, float v) {
    vec3 k = clamp(abs(fract(h + vec3(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, 0.0, 1.0);
    return v * mix(vec3(1.0), k, s);
}
vec2 rot(vec2 p, float a) { return vec2(cos(a) * p.x - sin(a) * p.y, sin(a) * p.x + cos(a) * p.y); }

float aspect;

// Where a point of this frame comes from in the last one.
vec2 source(vec2 uv) {
    vec2 c = vec2(CX, CY);
    vec2 q = (uv - c) * vec2(aspect, 1.0);
    float rad = length((uv - 0.5) * vec2(aspect, 1.0)) / (0.5 * length(vec2(aspect, 1.0)));
    float zoom = pow(max(ZOOM + ZOOM_RAD * rad, 0.05), u_k);
    q = rot(q, -(ROT + ROT_RAD * rad) * u_k) / zoom;
    q /= vec2(pow(SX, u_k), pow(SY, u_k));
    float wt = u_time * WARP_SPEED;
    q += WARP * 0.012 * u_k * vec2(sin(wt * 0.71 + q.y * 7.0 * WARP_SCALE + 1.3) + sin(wt * 0.37 + q.x * 3.0 * WARP_SCALE), cos(wt * 0.63 + q.x * 6.0 * WARP_SCALE + 0.4) + cos(wt * 0.29 + q.y * 3.7 * WARP_SCALE));
    float l = length(q);
    if (RIPPLE > 0.0005 && l > 0.0001) {
        q += q / l * RIPPLE * 0.006 * u_k * sin(l * 30.0 * RIPPLE_FREQ - u_time * 5.0);
    }
    return q / vec2(aspect, 1.0) + c - vec2(DX, DY) * u_k;
}

void main() {
    aspect = u_stage.x / u_stage.y;
    vec2 uv = v_uv;
    float px = 1.0 / u_stage.y;
    vec2 src = fract(source(uv));
    vec3 col = texture2D(u_prev, vec2(src.x, 1.0 - src.y)).rgb * pow(DECAY, u_k);
    vec2 pc = (uv - 0.5) * vec2(aspect, 1.0);
    col *= 1.0 - DARKEN_CENTER * 0.7 * exp(-dot(pc, pc) * 30.0);
    // Motion dots: where the field takes each point of a grid.
    if (MV_ALPHA > 0.001 && MV_X >= 1.0 && MV_Y >= 1.0) {
        vec2 n = vec2(MV_X, MV_Y);
        vec2 cell = (floor(uv * n) + 0.5) / n;
        vec2 to = cell + (cell - source(cell)) * 6.0;
        float d = length((uv - to) * vec2(aspect, 1.0));
        col += vec3(MV_ALPHA) * smoothstep(px * 2.2, px * 0.5, d);
    }
    // The wave.
    if (WAVE_ALPHA > 0.001) {
        vec3 wc = hsv(WAVE_HUE, WAVE_SAT, 1.0) * WAVE_ALPHA;
        float th = WAVE_THICK * px * 0.5;
        vec2 q = (uv - vec2(WAVE_X, WAVE_Y)) * vec2(aspect, 1.0);
        float s = WAVE_SCALE;
        float m = WAVE_MODE;
        float d = 1.0;
        if (m < 0.5) {
            d = abs(q.y + wave(uv.x) * 0.2 * s);
        } else if (m < 1.5) {
            float a = atan(q.y, q.x) / TAU + 0.5;
            d = abs(length(q) - (0.18 * s + wave(a) * 0.06 * s));
        } else if (m < 2.5) {
            // The wave against itself a quarter later: an oscilloscope's figures.
            vec2 last = vec2(wave(0.0), wave(0.25)) * 0.3 * s;
            for (int i = 1; i <= 48; i++) {
                float t = float(i) / 96.0;
                vec2 pt = vec2(wave(t), wave(t + 0.25)) * 0.3 * s;
                vec2 ab = pt - last;
                vec2 ap = q - last;
                float h = clamp(dot(ap, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
                d = min(d, length(ap - ab * h));
                last = pt;
            }
        } else if (m < 3.5) {
            float sx = abs(uv.x - 0.5) * 2.0;
            float h = spec(sx) * 0.35 * s;
            d = (-q.y > 0.0 && -q.y < h && fract(sx * 48.0) > 0.25) ? 0.0 : 1.0;
        } else if (m < 4.5) {
            float wv = wave(uv.x) * 0.12 * s;
            d = min(abs(q.y - 0.12 - wv), abs(q.y + 0.12 + wv));
        } else {
            float a = atan(q.y, q.x) / TAU + 0.5;
            float step_ = 0.06 * s + 0.001;
            float k = (length(q) - wave(a) * 0.02 * s) / step_ - a;
            d = length(q) < 0.45 ? abs(fract(k + 0.5) - 0.5) * step_ : 1.0;
        }
        col += wc * smoothstep(th + px, max(th - px * 0.5, 0.0), d);
    }
    // The shape (holding the whole last frame, when it does: pictures in pictures).
    if (SHAPE_ALPHA > 0.001 && SHAPE_RAD > 0.001) {
        vec2 q = rot((uv - vec2(SHAPE_X, SHAPE_Y)) * vec2(aspect, 1.0), -(SHAPE_ANG + u_time * SHAPE_SPIN));
        float seg = TAU / max(SHAPE_SIDES, 3.0);
        float a = atan(q.y, q.x);
        float dd = cos(floor(0.5 + a / seg) * seg - a) * length(q) - SHAPE_RAD * 0.5;
        if (dd < 0.0) {
            if (SHAPE_TEX > 0.5) {
                vec2 t = 0.5 + q / vec2(aspect, 1.0) / max(SHAPE_RAD, 0.05);
                col = mix(col, texture2D(u_prev, vec2(t.x, 1.0 - t.y)).rgb, SHAPE_ALPHA);
            } else {
                col += hsv(SHAPE_HUE, 0.7, 1.0) * SHAPE_ALPHA * 0.35;
            }
        }
        col += hsv(SHAPE_HUE, 0.5, 1.0) * SHAPE_ALPHA * smoothstep(px * 2.0, 0.0, abs(dd));
    }
    // Borders.
    if (BORDER_ALPHA > 0.001) {
        float e = min(min(uv.x, 1.0 - uv.x) * aspect, min(uv.y, 1.0 - uv.y));
        vec3 bc = hsv(BORDER_HUE, 0.7, 1.0);
        if (e < OUTER_BORDER) {
            col = mix(col, bc, BORDER_ALPHA);
        } else if (e < OUTER_BORDER + INNER_BORDER) {
            col = mix(col, bc * 0.5, BORDER_ALPHA);
        }
    }
    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, sr: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n).map(|i| (std::f32::consts::TAU * hz * i as f32 / sr).sin() * amp).collect()
    }

    #[test]
    fn a_tone_lights_its_band_of_the_spectrum() {
        let sr = 48000.0;
        for hz in [80.0, 1000.0, 8000.0] {
            let s = spectrum(&sine(hz, sr, 4096, 0.1), sr);
            let top = (0..SCOPE_W).max_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap();
            let band = |f: f32| ((f / 40.0).ln() / (16000.0f32 / 40.0).ln() * (SCOPE_W - 1) as f32).round() as usize;
            // Down low a band is narrower than the FFT's bins: near enough.
            assert!(top.abs_diff(band(hz)) <= if hz < 200.0 { 4 } else { 2 }, "{hz} Hz: band {top}, not {}", band(hz));
            assert!(s[top] > 0.5, "{hz} Hz is loud: {}", s[top]);
            assert!(s[band(hz * 4.0).min(SCOPE_W - 1)] < s[top] * 0.6, "two octaves up is quiet");
        }
        assert!(spectrum(&vec![0.0; 4096], sr).iter().all(|v| *v == 0.0), "silence");
        assert!(spectrum(&[0.5; 10], sr).iter().all(|v| *v == 0.0), "too little to hear");
    }

    #[test]
    fn the_wave_stands_still_and_fills_the_height() {
        let sr = 48000.0;
        let mut ears = Ears::default();
        // A quiet tone: drawn big all the same, starting where it rises through the middle.
        let tone = sine(375.0, sr, 4096, 0.05);
        ears.frame(&tone[..2048], 0, 0.0, 0.02, sr, 1.0);
        let a = ears.scope().wave;
        ears.frame(&tone[2048..], 0, 0.0, 0.02, sr, 1.0);
        let b = ears.scope().wave;
        assert!(a[0].abs_diff(128) <= 8 && a[2] > 128, "starts rising through the middle: {:?}", &a[..4]);
        assert!(a.iter().zip(b.iter()).all(|(x, y)| x.abs_diff(*y) <= 12), "the same place each frame");
        assert!(a.iter().copied().max().unwrap() > 220, "fills the height");
    }

    #[test]
    fn notes_light_and_fall_and_time_runs_with_speed() {
        let mut ears = Ears::default();
        ears.frame(&[], 1u128 << 60 | 1u128 << 64, 0.9, 0.02, 48000.0, 2.0);
        let s = ears.scope();
        assert!(s.notes[60] > 200 && s.notes[64] > 200 && s.notes[62] == 0);
        for _ in 0..50 {
            ears.frame(&[], 0, 0.0, 0.02, 48000.0, 2.0);
        }
        assert!(ears.scope().notes[60] < 60, "a second later: fading");
        assert!((ears.time - 51.0 * 0.02 * 2.0).abs() < 1e-6, "twice as fast: {}", ears.time);
        let rgba = s.rgba();
        assert_eq!(rgba.len(), SCOPE_W * 4 * 4);
        assert_eq!(rgba[(2 * SCOPE_W + 60) * 4], s.notes[60], "the notes' row");
    }

    #[test]
    fn the_shader_has_every_scene_and_knob() {
        for knob in ["A_BURST", "A_WARP", "A_DENSITY", "A_GLOW", "A_ORBIT", "A_FRACTURE", "P_VISUALS"] {
            assert!(SHADER.contains(knob), "{knob}");
        }
        for f in ["ridges(", "lattice(", "swarm(", "data(", "tunnel(", "scope(", "contours(", "shards(", "notegrid(", "strings(", "milk("] {
            assert!(SHADER.matches(f).count() >= 2, "{f} drawn");
        }
        assert_eq!(krabton_engine::video::VISUAL_SCENES.len(), 11);
        // Every Milk value the shaders use has its name.
        let names = milk_defines();
        for word in ["ZOOM", "WAVE_MODE", "SHAPE_TEX", "ECHO_ORIENT", "SOLARIZE", "MV_ALPHA", "DARKEN_CENTER"] {
            assert!(names.contains(&format!("#define {word} ")), "{word}");
            assert!(MILK_SHADER.contains(word) || SHADER.contains(word), "{word} used");
        }
    }
}
