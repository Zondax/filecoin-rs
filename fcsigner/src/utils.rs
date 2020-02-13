use std::{fmt, fmt::Write};

/// DecoderError
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexDecodeError {
    /// String length is invalid and could not be decoded
    InvalidLength,
    /// hex value could not be decoded
    ParseInt(std::num::ParseIntError),
}

impl From<std::num::ParseIntError> for HexDecodeError {
    fn from(e: std::num::ParseIntError) -> Self {
        HexDecodeError::ParseInt(e)
    }
}

impl fmt::Display for HexDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HexDecodeError::InvalidLength => "Invalid length 0 or odd number of characters".fmt(f),
            HexDecodeError::ParseInt(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for HexDecodeError {}

/// convert array to hexstring
pub fn to_hex_string(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &byte in data {
        write!(&mut s, "{:02x}", byte).expect("ERR");
    }
    s
}

/// convert hexstring to array
pub fn from_hex_string(s: &str) -> Result<Vec<u8>, HexDecodeError> {
    if s.is_empty() || s.len() % 2 != 0 {
        return Err(HexDecodeError::InvalidLength);
    }

    let mut vec = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let v = u8::from_str_radix(&s[i..i + 2], 16)?;
        vec.push(v);
    }
    Ok(vec)
}