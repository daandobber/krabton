//! A small OSC 1.0 encoder/decoder (int32, float32 and string arguments;
//! bundles are flattened). Enough for serialosc.

#[derive(Clone, Debug, PartialEq)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    Str(String),
}

impl OscArg {
    pub fn as_int(&self) -> Option<i32> {
        match self {
            OscArg::Int(i) => Some(*i),
            OscArg::Float(f) => Some(*f as i32),
            OscArg::Str(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            OscArg::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OscMessage {
    pub addr: String,
    pub args: Vec<OscArg>,
}

impl OscMessage {
    pub fn new(addr: impl Into<String>, args: Vec<OscArg>) -> Self {
        Self { addr: addr.into(), args }
    }

    pub fn int(&self, i: usize) -> Option<i32> {
        self.args.get(i).and_then(|a| a.as_int())
    }

    pub fn str(&self, i: usize) -> Option<&str> {
        self.args.get(i).and_then(|a| a.as_str())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.args.len() * 4);
        push_str(&mut out, &self.addr);
        let mut tags = String::from(",");
        for a in &self.args {
            tags.push(match a {
                OscArg::Int(_) => 'i',
                OscArg::Float(_) => 'f',
                OscArg::Str(_) => 's',
            });
        }
        push_str(&mut out, &tags);
        for a in &self.args {
            match a {
                OscArg::Int(i) => out.extend_from_slice(&i.to_be_bytes()),
                OscArg::Float(f) => out.extend_from_slice(&f.to_be_bytes()),
                OscArg::Str(s) => push_str(&mut out, s),
            }
        }
        out
    }
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn read_str(data: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    let end = start + data.get(start..)?.iter().position(|&b| b == 0)?;
    let s = std::str::from_utf8(&data[start..end]).ok()?.to_string();
    *pos = (end + 4) & !3;
    Some(s)
}

fn read_i32(data: &[u8], pos: &mut usize) -> Option<i32> {
    let b: [u8; 4] = data.get(*pos..*pos + 4)?.try_into().ok()?;
    *pos += 4;
    Some(i32::from_be_bytes(b))
}

/// Decodes a packet into messages (bundles are flattened).
pub fn decode(data: &[u8]) -> Vec<OscMessage> {
    let mut out = Vec::new();
    decode_into(data, &mut out, 0);
    out
}

fn decode_into(data: &[u8], out: &mut Vec<OscMessage>, depth: usize) {
    if depth > 8 || data.is_empty() {
        return;
    }
    if data.starts_with(b"#bundle\0") {
        let mut pos = 16; // "#bundle\0" + 8-byte time tag
        while pos + 4 <= data.len() {
            let Some(size) = read_i32(data, &mut pos) else { break };
            let size = size.max(0) as usize;
            if pos + size > data.len() {
                break;
            }
            decode_into(&data[pos..pos + size], out, depth + 1);
            pos += size;
        }
        return;
    }
    if let Some(m) = decode_message(data) {
        out.push(m);
    }
}

fn decode_message(data: &[u8]) -> Option<OscMessage> {
    let mut pos = 0;
    let addr = read_str(data, &mut pos)?;
    if !addr.starts_with('/') {
        return None;
    }
    if pos >= data.len() {
        return Some(OscMessage { addr, args: Vec::new() });
    }
    let tags = read_str(data, &mut pos)?;
    let mut args = Vec::new();
    for t in tags.chars().skip(1) {
        match t {
            'i' => args.push(OscArg::Int(read_i32(data, &mut pos)?)),
            'f' => args.push(OscArg::Float(f32::from_bits(read_i32(data, &mut pos)? as u32))),
            's' | 'S' => args.push(OscArg::Str(read_str(data, &mut pos)?)),
            'T' | 'F' | 'N' | 'I' => {}
            _ => return None,
        }
    }
    Some(OscMessage { addr, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let m = OscMessage::new("/serialosc/list", vec![OscArg::Str("127.0.0.1".into()), OscArg::Int(13001), OscArg::Float(0.5)]);
        let bytes = m.encode();
        assert_eq!(bytes.len() % 4, 0);
        assert_eq!(decode(&bytes), vec![m]);
    }

    #[test]
    fn known_encoding() {
        // "/a" + pad, ",i" + pad, 1
        let m = OscMessage::new("/a", vec![OscArg::Int(1)]);
        assert_eq!(m.encode(), vec![b'/', b'a', 0, 0, b',', b'i', 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn bundles_and_garbage() {
        let a = OscMessage::new("/x", vec![OscArg::Int(7)]).encode();
        let mut bundle = b"#bundle\0".to_vec();
        bundle.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
        bundle.extend_from_slice(&(a.len() as i32).to_be_bytes());
        bundle.extend_from_slice(&a);
        let msgs = decode(&bundle);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].int(0), Some(7));
        assert!(decode(&[1, 2, 3]).is_empty());
        assert!(decode(b"/a\0\0,i\0\0\0").is_empty(), "truncated int");
    }
}
