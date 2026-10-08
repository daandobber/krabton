//! Generic parameter descriptions. Every instrument and effect exposes its
//! parameters as a flat `Vec<f32>` plus a static table of [`ParamSpec`]s, so
//! the UI can build knobs for any device without knowing about it.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    None,
    Db,
    Hz,
    Ms,
    Sec,
    Percent,
    Semi,
    Cents,
    Ratio,
    Octaves,
    Pan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    Linear,
    Log,
    Int,
    Choice(&'static [&'static str]),
    Toggle,
}

#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub name: &'static str,
    pub group: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub kind: ParamKind,
    pub unit: Unit,
}

impl ParamSpec {
    pub const fn lin(name: &'static str, group: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> Self {
        Self { name, group, min, max, default, kind: ParamKind::Linear, unit }
    }

    pub const fn log(name: &'static str, group: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> Self {
        Self { name, group, min, max, default, kind: ParamKind::Log, unit }
    }

    pub const fn int(name: &'static str, group: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> Self {
        Self { name, group, min, max, default, kind: ParamKind::Int, unit }
    }

    pub const fn choice(name: &'static str, group: &'static str, options: &'static [&'static str], default: usize) -> Self {
        Self {
            name,
            group,
            min: 0.0,
            max: (options.len() - 1) as f32,
            default: default as f32,
            kind: ParamKind::Choice(options),
            unit: Unit::None,
        }
    }

    pub const fn toggle(name: &'static str, group: &'static str, default: bool) -> Self {
        Self {
            name,
            group,
            min: 0.0,
            max: 1.0,
            default: if default { 1.0 } else { 0.0 },
            kind: ParamKind::Toggle,
            unit: Unit::None,
        }
    }

    /// Map a value to 0..1 for knob display.
    pub fn to_norm(&self, v: f32) -> f32 {
        let v = v.clamp(self.min, self.max);
        match self.kind {
            ParamKind::Log => (v / self.min).ln() / (self.max / self.min).ln(),
            _ => {
                if self.max > self.min {
                    (v - self.min) / (self.max - self.min)
                } else {
                    0.0
                }
            }
        }
    }

    /// Map 0..1 back to a value, rounding discrete kinds.
    pub fn from_norm(&self, n: f32) -> f32 {
        let n = n.clamp(0.0, 1.0);
        let v = match self.kind {
            ParamKind::Log => self.min * (self.max / self.min).powf(n),
            _ => self.min + (self.max - self.min) * n,
        };
        self.clamp(v)
    }

    pub fn clamp(&self, v: f32) -> f32 {
        let v = v.clamp(self.min, self.max);
        match self.kind {
            ParamKind::Int | ParamKind::Choice(_) | ParamKind::Toggle => v.round(),
            _ => v,
        }
    }

    pub fn is_discrete(&self) -> bool {
        matches!(self.kind, ParamKind::Int | ParamKind::Choice(_) | ParamKind::Toggle)
    }

    pub fn format(&self, v: f32) -> String {
        match self.kind {
            ParamKind::Choice(opts) => return opts.get(v.round() as usize).copied().unwrap_or("?").to_string(),
            ParamKind::Toggle => return if v >= 0.5 { "On".into() } else { "Off".into() },
            _ => {}
        }
        match self.unit {
            Unit::None => {
                if self.kind == ParamKind::Int {
                    format!("{}", v.round() as i32)
                } else {
                    format!("{v:.2}")
                }
            }
            Unit::Db => {
                if v <= self.min && self.min <= -60.0 {
                    "-inf dB".into()
                } else {
                    format!("{v:.1} dB")
                }
            }
            Unit::Hz => {
                if v >= 1000.0 {
                    format!("{:.2} kHz", v / 1000.0)
                } else if v >= 100.0 {
                    format!("{v:.0} Hz")
                } else {
                    format!("{v:.2} Hz")
                }
            }
            Unit::Ms => {
                if v >= 1000.0 {
                    format!("{:.2} s", v / 1000.0)
                } else if v >= 10.0 {
                    format!("{v:.0} ms")
                } else {
                    format!("{v:.1} ms")
                }
            }
            Unit::Sec => {
                if v < 1.0 {
                    format!("{:.0} ms", v * 1000.0)
                } else {
                    format!("{v:.2} s")
                }
            }
            Unit::Percent => format!("{:.0} %", v * 100.0),
            Unit::Semi => format!("{:+} st", v.round() as i32),
            Unit::Cents => format!("{:+.0} ct", v),
            Unit::Ratio => format!("{v:.1} : 1"),
            Unit::Octaves => format!("{v:+.2} oct"),
            Unit::Pan => {
                let p = (v * 50.0).round() as i32;
                if p == 0 {
                    "C".into()
                } else if p < 0 {
                    format!("{}L", -p)
                } else {
                    format!("{p}R")
                }
            }
        }
    }
}

pub fn defaults(specs: &[ParamSpec]) -> Vec<f32> {
    specs.iter().map(|s| s.default).collect()
}

/// Make a parameter vector match its spec table (used after loading files
/// written by older versions).
pub fn fix_len(params: &mut Vec<f32>, specs: &[ParamSpec]) {
    if params.len() > specs.len() {
        params.truncate(specs.len());
    }
    while params.len() < specs.len() {
        params.push(specs[params.len()].default);
    }
    for (p, s) in params.iter_mut().zip(specs) {
        if !p.is_finite() {
            *p = s.default;
        }
        *p = s.clamp(*p);
    }
}
