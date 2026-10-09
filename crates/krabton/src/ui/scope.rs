//! Waveform views: the live wave in every instrument panel and the
//! Oscilloscope device.
//!
//! Periodic signals are drawn locked to their pitch: the period is found
//! with a normalized square difference function, and the window starts
//! where the fundamental rises through zero, so the wave stands still
//! however complex it is. Other signals get a classic triggered time base.

use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::effects::{SCOPE_AUTO, SCOPE_CHANNEL, SCOPE_CYCLES, SCOPE_FREEZE, SCOPE_GAIN, SCOPE_GLOW, SCOPE_MODE, SCOPE_TIME, SCOPE_TRIGGER};
use krabton_engine::engine::EngineStatus;
use krabton_engine::scope::ScopeBuffer;
use std::collections::VecDeque;
use std::f32::consts::PI;

const SCREEN: Color32 = Color32::from_rgb(0x0b, 0x11, 0x0e);
const GRID: Color32 = Color32::from_rgb(0x1c, 0x2a, 0x22);
const GRID_MID: Color32 = Color32::from_rgb(0x2a, 0x3d, 0x31);
const TRACES: [Color32; 2] = [PLAY, SELECT];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Locked to the pitch, a number of cycles wide.
    Wave,
    /// A fixed time window with a trigger.
    Time,
    /// Left against right (Lissajous figures).
    Xy,
    /// Mid up, side across: mono is a vertical line.
    Vector,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trigger {
    Rising,
    Falling,
    Free,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Mid,
    Stereo,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct ScopeOpts {
    pub mode: Mode,
    pub cycles: usize,
    /// Window of the Time, XY and Vector modes, in seconds.
    pub time: f32,
    pub trigger: Trigger,
    pub channel: Channel,
    pub auto_gain: bool,
    /// Linear gain when `auto_gain` is off.
    pub gain: f32,
    /// Afterglow, 0..1.
    pub glow: f32,
    pub freeze: bool,
    /// Small view: no labels, and the last wave stays visible in silence.
    pub compact: bool,
}

impl ScopeOpts {
    /// Settings from the Oscilloscope device's parameters.
    pub fn from_params(p: &[f32]) -> Self {
        let get = |i: usize| p.get(i).copied().unwrap_or(0.0);
        Self {
            mode: match get(SCOPE_MODE).round() as i32 {
                1 => Mode::Time,
                2 => Mode::Xy,
                3 => Mode::Vector,
                _ => Mode::Wave,
            },
            cycles: (get(SCOPE_CYCLES).round() as usize).clamp(1, 8),
            time: (get(SCOPE_TIME) / 1000.0).clamp(0.001, 2.0),
            trigger: match get(SCOPE_TRIGGER).round() as i32 {
                1 => Trigger::Falling,
                2 => Trigger::Free,
                _ => Trigger::Rising,
            },
            channel: match get(SCOPE_CHANNEL).round() as i32 {
                1 => Channel::Stereo,
                2 => Channel::Left,
                3 => Channel::Right,
                _ => Channel::Mid,
            },
            auto_gain: get(SCOPE_AUTO) >= 0.5,
            gain: krabton_engine::dsp::db_to_gain(get(SCOPE_GAIN)),
            glow: get(SCOPE_GLOW).clamp(0.0, 1.0),
            freeze: get(SCOPE_FREEZE) >= 0.5,
            compact: false,
        }
    }
}

/// Normalized square difference of `y` at lag `tau` (1 = identical).
fn nsdf(y: &[f32], tau: usize) -> f32 {
    if tau >= y.len() {
        return 0.0;
    }
    let (mut acf, mut m) = (0.0f32, 0.0f32);
    for j in 0..y.len() - tau {
        acf += y[j] * y[j + tau];
        m += y[j] * y[j] + y[j + tau] * y[j + tau];
    }
    if m > 1e-12 {
        2.0 * acf / m
    } else {
        0.0
    }
}

/// The pitch period of `x` in samples, if it is periodic enough: a coarse
/// search on a decimated copy of the last 4096 samples (down to about 23 Hz
/// at 48 kHz), then of the last 8192 (down to about 12 Hz), refined at the
/// full rate.
pub fn detect_period(x: &[f32]) -> Option<f32> {
    period_in(x, 4096, 4).or_else(|| period_in(x, 8192, 8))
}

fn period_in(x: &[f32], window: usize, decimation: usize) -> Option<f32> {
    #[allow(non_snake_case)]
    let D = decimation;
    let x = &x[x.len().saturating_sub(window)..];
    let n = x.len() / D;
    if n < 64 {
        return None;
    }
    let dec: Vec<f32> = (0..n).map(|i| x[i * D..i * D + D].iter().sum::<f32>() / D as f32).collect();
    let max_lag = n / 2;
    let curve: Vec<f32> = (0..max_lag).map(|t| nsdf(&dec, t)).collect();
    // The highest point of every positive lobe after the one at lag zero.
    let mut t = 1;
    while t < max_lag && curve[t] > 0.0 {
        t += 1;
    }
    let mut maxima: Vec<(usize, f32)> = Vec::new();
    let mut best: Option<(usize, f32)> = None;
    for (k, &c) in curve.iter().enumerate().skip(t) {
        if c > 0.0 {
            if best.map_or(true, |b| c > b.1) {
                best = Some((k, c));
            }
        } else if let Some(b) = best.take() {
            maxima.push(b);
        }
    }
    // A lobe cut off by the end of the range counts only if it already
    // turned down: a curve still rising at the edge is not a peak.
    if let Some(b) = best {
        if b.0 + 2 < max_lag && curve[max_lag - 1] < 0.9 * b.1 {
            maxima.push(b);
        }
    }
    let top = maxima.iter().map(|m| m.1).fold(0.0, f32::max);
    if top < 0.5 {
        return None;
    }
    let coarse = maxima.iter().find(|m| m.1 >= 0.9 * top)?.0 * D;
    // Refine at the full rate, with a parabola through the best lag.
    let y = &x[x.len().saturating_sub((3 * coarse).max(2048))..];
    let lo = coarse.saturating_sub(D + 1).max(2);
    let hi = (coarse + D + 1).min(y.len() / 2);
    if lo >= hi {
        return Some(coarse as f32);
    }
    let (t, v) = (lo..=hi).map(|t| (t, nsdf(y, t))).fold((lo, f32::MIN), |b, c| if c.1 > b.1 { c } else { b });
    if t == lo || t == hi {
        return Some(t as f32);
    }
    let (a, e) = (nsdf(y, t - 1), nsdf(y, t + 1));
    let d = a - 2.0 * v + e;
    let shift = if d.abs() > 1e-9 { (0.5 * (a - e) / d).clamp(-1.0, 1.0) } else { 0.0 };
    Some(t as f32 + shift)
}

/// Start (fractional index) of a window of `span` samples that begins where
/// the fundamental of period `period` rises through zero, as late in `x` as
/// possible.
pub fn locked_start(x: &[f32], period: f32, span: f32) -> f32 {
    let len = x.len() as f32;
    let periods = (span / period).ceil() + 1.0;
    let s0 = (len - periods * period).max(0.0).ceil() as usize;
    // Phase of the fundamental by one DFT bin over whole periods.
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (k, &v) in x[s0..].iter().enumerate() {
        let th = 2.0 * PI * k as f32 / period;
        re += v * th.cos();
        im -= v * th.sin();
    }
    let phi = im.atan2(re);
    let frac = (-0.25 - phi / (2.0 * PI)).rem_euclid(1.0);
    let mut t0 = s0 as f32 + frac * period;
    while t0 + period + span <= len - 1.0 {
        t0 += period;
    }
    while t0 + span > len - 1.0 && t0 >= period {
        t0 -= period;
    }
    t0
}

/// Start of a window of `span` samples at the latest trigger point.
pub fn trigger_start(x: &[f32], span: f32, trigger: Trigger) -> f32 {
    let len = x.len();
    let free = (len as f32 - 1.0 - span).max(0.0);
    if trigger == Trigger::Free || len < 4 {
        return free;
    }
    let last = free as usize;
    let first = last.saturating_sub((span as usize).max(256)).max(1);
    let peak = x[first..last.max(first + 1).min(len)].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let h = 0.05 * peak;
    let sign = if trigger == Trigger::Rising { 1.0 } else { -1.0 };
    let mut armed_at = None;
    // Walk back: remember the latest crossing whose approach dipped past
    // the hysteresis band.
    for i in (first..=last).rev() {
        let (a, b) = (x[i - 1] * sign, x[i] * sign);
        if a < 0.0 && b >= 0.0 {
            let dipped = x[i.saturating_sub(64)..i].iter().any(|v| v * sign < -h);
            if dipped {
                armed_at = Some(i - 1);
                break;
            }
        }
    }
    match armed_at {
        Some(i) => {
            let (a, b) = (x[i], x[i + 1]);
            i as f32 + if a != b { a / (a - b) } else { 0.0 }
        }
        None => free,
    }
}

/// One trace from `t0` over `span` samples as (0..1, value) points, one or
/// two per column (min and max where a column covers several samples).
fn columns(x: &[f32], t0: f32, span: f32, cols: usize) -> Vec<[f32; 2]> {
    let n = x.len();
    let mut pts = Vec::with_capacity(cols * 2 + 2);
    if n < 2 {
        return pts;
    }
    let at = |t: f32| {
        let t = t.clamp(0.0, (n - 1) as f32);
        let i = t as usize;
        let f = t - i as f32;
        let b = x[(i + 1).min(n - 1)];
        x[i] + (b - x[i]) * f
    };
    let step = span / cols as f32;
    let mut last = at(t0);
    for c in 0..=cols {
        let u = c as f32 / cols as f32;
        let a = t0 + c as f32 * step;
        if step <= 1.5 || c == cols {
            last = at(a);
            pts.push([u, last]);
        } else {
            let i0 = (a.max(0.0) as usize).min(n - 1);
            let i1 = ((a + step) as usize).clamp(i0 + 1, n);
            let (lo, hi) = x[i0..i1].iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            if (hi - last).abs() < (lo - last).abs() {
                pts.push([u, hi]);
                pts.push([u, lo]);
                last = lo;
            } else {
                pts.push([u, lo]);
                pts.push([u, hi]);
                last = hi;
            }
        }
    }
    pts
}

/// What was drawn in one refresh.
#[derive(Clone, Default)]
struct Frame {
    /// (points, colour index); time traces as (u, value), XY as (x, y).
    traces: Vec<(Vec<[f32; 2]>, usize)>,
    xy: bool,
}

#[derive(Clone, Default)]
struct Mem {
    history: VecDeque<Frame>,
    last_good: Option<Frame>,
    gain: f32,
    readout: String,
    level: String,
}

/// Reads and analyses the buffer into a frame, its peak and a readout.
fn capture(buf: &ScopeBuffer, sr: f32, o: &ScopeOpts, cols: usize) -> (Frame, f32, String) {
    let (mut l, mut r) = (Vec::new(), Vec::new());
    let span_t = (o.time * sr).max(16.0);
    let want = match o.mode {
        Mode::Wave => 16384,
        Mode::Time => (2.0 * span_t) as usize + 64,
        Mode::Xy | Mode::Vector => span_t as usize,
    };
    buf.read(want.min(buf.len()), &mut l, &mut r);
    if l.len() < 64 {
        return (Frame::default(), 0.0, String::new());
    }
    let stereo = !r.is_empty();
    let mid: Vec<f32> = if stereo { l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)).collect() } else { l.clone() };
    if matches!(o.mode, Mode::Xy | Mode::Vector) {
        let (rr, n) = (if stereo { &r } else { &l }, l.len());
        let stride = n.div_ceil(3000).max(1);
        let mut pts = Vec::with_capacity(n / stride + 1);
        let mut peak = 0.0f32;
        for i in (0..n).step_by(stride) {
            let (a, b) = (l[i], rr[i]);
            let p = if o.mode == Mode::Xy { [a, b] } else { [(a - b) * 0.7071, (a + b) * 0.7071] };
            peak = peak.max(p[0].abs()).max(p[1].abs());
            pts.push(p);
        }
        return (Frame { traces: vec![(pts, 0)], xy: true }, peak, String::new());
    }
    let len = mid.len() as f32;
    let mut readout = String::new();
    let (t0, span) = match o.mode {
        Mode::Wave => match detect_period(&mid) {
            Some(per) => {
                let mut cycles = o.cycles.max(1);
                while cycles > 1 && (cycles as f32 + 2.0) * per > len {
                    cycles -= 1;
                }
                let span = cycles as f32 * per;
                let f = sr / per;
                let midi = 69.0 + 12.0 * (f / 440.0).log2();
                let note = midi.round().clamp(0.0, 127.0);
                readout = format!("{f:.1} Hz  {} {:+.0}c", krabton_engine::note_name(note as u8), (midi - note) * 100.0);
                (locked_start(&mid, per, span), span)
            }
            None => {
                let span = (0.03 * sr).min(len / 2.0);
                (trigger_start(&mid, span, Trigger::Rising), span)
            }
        },
        _ => {
            let span = span_t.min(len - 2.0);
            (trigger_start(&mid, span, o.trigger), span)
        }
    };
    let chans: Vec<(&[f32], usize)> = if !stereo {
        vec![(l.as_slice(), 0)]
    } else {
        match o.channel {
            Channel::Stereo => vec![(l.as_slice(), 0), (r.as_slice(), 1)],
            Channel::Left => vec![(l.as_slice(), 0)],
            Channel::Right => vec![(r.as_slice(), 1)],
            Channel::Mid => vec![(mid.as_slice(), 0)],
        }
    };
    let i0 = (t0.max(0.0) as usize).min(mid.len() - 1);
    let i1 = ((t0 + span) as usize + 1).min(mid.len());
    let peak = chans.iter().map(|(x, _)| x[i0..i1].iter().fold(0.0f32, |m, v| m.max(v.abs()))).fold(0.0, f32::max);
    let traces = chans.into_iter().map(|(x, c)| (columns(x, t0, span, cols), c)).collect();
    (Frame { traces, xy: false }, peak, readout)
}

fn draw_frame(p: &egui::Painter, rect: Rect, f: &Frame, gain: f32, alpha: f32, glow: bool) {
    let c = rect.center();
    let half_h = rect.height() * 0.5 * 0.92;
    let half_xy = rect.width().min(rect.height()) * 0.5 * 0.92;
    for (pts, ci) in &f.traces {
        if pts.len() < 2 {
            continue;
        }
        let pos: Vec<Pos2> = if f.xy {
            pts.iter().map(|q| pos2(c.x + (q[0] * gain).clamp(-1.1, 1.1) * half_xy, c.y - (q[1] * gain).clamp(-1.1, 1.1) * half_xy)).collect()
        } else {
            pts.iter().map(|q| pos2(rect.left() + q[0] * rect.width(), c.y - (q[1] * gain).clamp(-1.1, 1.1) * half_h)).collect()
        };
        let color = TRACES[*ci % TRACES.len()];
        if glow {
            p.add(Shape::line(pos.clone(), Stroke::new(4.0_f32, color.gamma_multiply(0.18 * alpha))));
        }
        p.add(Shape::line(pos, Stroke::new(1.4_f32, color.gamma_multiply(alpha))));
    }
}

/// Draws a scope into `rect` (already allocated by the caller) and returns
/// the click response.
pub fn scope_view(ui: &mut Ui, rect: Rect, id: Id, buf: Option<&ScopeBuffer>, sr: f32, o: &ScopeOpts) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, SCREEN);
    let xy = matches!(o.mode, Mode::Xy | Mode::Vector);
    // Graticule.
    let (cx, cy) = (rect.center().x, rect.center().y);
    if xy {
        let rad = rect.width().min(rect.height()) * 0.46;
        p.circle_stroke(rect.center(), rad, Stroke::new(1.0_f32, GRID));
        p.line_segment([pos2(cx - rad, cy), pos2(cx + rad, cy)], Stroke::new(1.0_f32, GRID_MID));
        p.line_segment([pos2(cx, cy - rad), pos2(cx, cy + rad)], Stroke::new(1.0_f32, GRID_MID));
    } else {
        let (nx, ny) = if o.compact { (4, 4) } else { (8, 6) };
        for i in 1..nx {
            let x = rect.left() + rect.width() * i as f32 / nx as f32;
            p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, GRID));
        }
        for i in 1..ny {
            let y = rect.top() + rect.height() * i as f32 / ny as f32;
            p.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, if i * 2 == ny { GRID_MID } else { GRID }));
        }
    }

    let mut mem: Mem = ui.ctx().memory_mut(|m| m.data.get_temp::<Mem>(id)).unwrap_or_default();
    if mem.gain <= 0.0 {
        mem.gain = 1.0;
    }
    let keep = (o.glow * 6.0).round() as usize;
    let mut silent = false;
    if !o.freeze {
        if let Some(buf) = buf {
            let cols = (rect.width() as usize).clamp(32, 1024);
            let (frame, peak, readout) = capture(buf, sr, o, cols);
            silent = peak < 1e-4;
            if o.auto_gain && !silent {
                let target = (0.9 / peak).clamp(0.5, 2000.0);
                mem.gain = if target < mem.gain { target } else { mem.gain + (target - mem.gain) * 0.06 };
            }
            if !silent {
                mem.readout = readout;
                mem.level = format!("{:.1} dB", 20.0 * peak.log10());
                mem.last_good = Some(frame.clone());
            } else {
                mem.readout.clear();
                mem.level = "silent".into();
            }
            mem.history.push_back(frame);
            while mem.history.len() > keep + 1 {
                mem.history.pop_front();
            }
        }
    }
    let gain = if o.auto_gain { mem.gain } else { o.gain };
    if silent && o.compact {
        // Keep the last wave on screen, dimmed, until the next note.
        if let Some(f) = &mem.last_good {
            draw_frame(&p, rect, f, gain, 0.3, false);
        }
    } else {
        let n = mem.history.len();
        for (k, f) in mem.history.iter().enumerate() {
            let age = n - 1 - k;
            let alpha = if age == 0 { 1.0 } else { 0.4 * (1.0 - age as f32 / (keep + 1) as f32).powi(2) };
            draw_frame(&p, rect, f, gain, alpha, age == 0);
        }
    }
    // Labels.
    let font = small();
    if o.compact {
        if !mem.readout.is_empty() {
            let note = mem.readout.split("  ").nth(1).unwrap_or("").to_string();
            p.text(rect.left_bottom() + vec2(4.0, -3.0), Align2::LEFT_BOTTOM, note, font, TEXT_DIM);
        }
    } else {
        let what = match o.mode {
            Mode::Wave => format!("WAVE  {} cycle{}", o.cycles, if o.cycles == 1 { "" } else { "s" }),
            Mode::Time => format!("TIME  {:.0} ms  {}", o.time * 1000.0, ["rising", "falling", "free"][o.trigger as usize]),
            Mode::Xy => "XY  left / right".into(),
            Mode::Vector => "VECTOR  mid / side".into(),
        };
        p.text(rect.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, what, font.clone(), TEXT_DIM);
        p.text(rect.right_top() + vec2(-5.0, 4.0), Align2::RIGHT_TOP, &mem.readout, font.clone(), TEXT);
        let gain_label = if o.auto_gain { format!("{}   auto x{:.1}", mem.level, gain) } else { format!("{}   x{:.1}", mem.level, gain) };
        p.text(rect.right_bottom() + vec2(-5.0, -3.0), Align2::RIGHT_BOTTOM, gain_label, font.clone(), TEXT_DIM);
        if o.freeze {
            p.text(rect.left_bottom() + vec2(5.0, -3.0), Align2::LEFT_BOTTOM, "FROZEN", font, ACCENT);
        }
        if buf.is_none() {
            p.text(rect.center(), Align2::CENTER_CENTER, "no signal yet", small(), TEXT_DIM);
        }
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(id, mem));
    if !o.freeze {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
    resp
}

/// The live wave of a track's instrument, for its device panel. Click to
/// show 1, 2 or 4 cycles.
pub fn instrument_wave(ui: &mut Ui, status: &EngineStatus, ti: usize, track_id: u64, height: f32) {
    let Some(ts) = status.tracks.get(ti) else { return };
    ts.watch_wave();
    let id = Id::new(("inst_wave", track_id));
    let cycles_id = id.with("cycles");
    let cycles = ui.ctx().memory(|m| m.data.get_temp::<usize>(cycles_id)).unwrap_or(2);
    let (rect, _) = ui.allocate_exact_size(vec2(150.0, height.max(60.0)), Sense::hover());
    let opts = ScopeOpts {
        mode: Mode::Wave,
        cycles,
        time: 0.03,
        trigger: Trigger::Rising,
        channel: Channel::Mid,
        auto_gain: true,
        gain: 1.0,
        glow: 0.35,
        freeze: false,
        compact: true,
    };
    let resp = scope_view(ui, rect, id, Some(&ts.wave), status.sample_rate.load(), &opts);
    if resp.clicked() {
        let next = match cycles {
            1 => 2,
            2 => 4,
            _ => 1,
        };
        ui.ctx().memory_mut(|m| m.data.insert_temp(cycles_id, next));
    }
    resp.on_hover_text(format!("Live wave of this instrument, locked to its pitch ({cycles} cycle{}). Click for 1, 2 or 4 cycles.", if cycles == 1 { "" } else { "s" }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saw(freq: f32, sr: f32, n: usize, phase: f32) -> Vec<f32> {
        (0..n).map(|i| ((i as f32 * freq / sr + phase).fract() * 2.0 - 1.0) * 0.5).collect()
    }

    #[test]
    fn finds_the_period_of_harmonic_and_low_signals() {
        let sr = 48000.0;
        for f in [14.0f32, 30.0, 55.0, 110.0, 220.0, 440.0, 1000.0, 2000.0] {
            let x = saw(f, sr, 8192, 0.3);
            let p = detect_period(&x).unwrap_or_else(|| panic!("{f} Hz not found"));
            assert!((sr / p - f).abs() < f * 0.003, "{f} Hz measured {}", sr / p);
        }
        // A square wave and a wave with a weak fundamental.
        let sq: Vec<f32> = (0..8192).map(|i| if (i as f32 * 150.0 / sr).fract() < 0.5 { 0.4 } else { -0.4 }).collect();
        assert!((sr / detect_period(&sq).unwrap() - 150.0).abs() < 0.5);
        let rich: Vec<f32> = (0..8192).map(|i| {
            let t = i as f32 / sr;
            0.1 * (2.0 * PI * 200.0 * t).sin() + 0.5 * (2.0 * PI * 600.0 * t).sin() + 0.4 * (2.0 * PI * 1000.0 * t).sin()
        }).collect();
        assert!((sr / detect_period(&rich).unwrap() - 200.0).abs() < 1.0);
        // Noise is not periodic.
        let mut s = 1u32;
        let noise: Vec<f32> = (0..8192).map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32 / u32::MAX as f32) - 0.5
        }).collect();
        assert!(detect_period(&noise).is_none());
    }

    #[test]
    fn locked_window_starts_at_the_same_phase_whatever_the_buffer_offset() {
        let sr = 48000.0;
        let f = 220.0;
        let per = sr / f;
        for offset in [0.0f32, 0.17, 0.5, 0.83] {
            // A wave with two zero crossings per cycle in the same direction.
            let x: Vec<f32> = (0..6000).map(|i| {
                let ph = 2.0 * PI * (i as f32 / per + offset);
                ph.sin() + 0.8 * (3.0 * ph).sin()
            }).collect();
            let t0 = locked_start(&x, per, 2.0 * per);
            assert!(t0 + 2.0 * per <= x.len() as f32);
            // The fundamental's phase at t0 is a rising zero crossing.
            let phase = (t0 / per + offset).rem_euclid(1.0);
            assert!(phase < 0.01 || phase > 0.99, "offset {offset}: phase {phase}");
        }
    }

    #[test]
    fn trigger_finds_the_latest_rising_edge() {
        let x: Vec<f32> = (0..4000).map(|i| (2.0 * PI * i as f32 / 100.0).sin()).collect();
        let t0 = trigger_start(&x, 500.0, Trigger::Rising);
        assert!(t0 + 500.0 <= 3999.0);
        let i = t0.round() as usize;
        assert!(x[i].abs() < 0.05 && x[i + 2] > x[i], "not at a rising crossing: {t0}");
        let t1 = trigger_start(&x, 500.0, Trigger::Falling);
        let j = t1.round() as usize;
        assert!(x[j].abs() < 0.05 && x[j + 2] < x[j]);
    }

    #[test]
    fn columns_keep_peaks_when_zoomed_out() {
        let x: Vec<f32> = (0..1000).map(|i| if i == 500 { 1.0 } else { 0.0 }).collect();
        let pts = columns(&x, 0.0, 1000.0, 50);
        assert!(pts.iter().any(|p| p[1] == 1.0), "a one-sample spike survives");
    }
}
