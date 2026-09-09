//! Bounded, allocation-free JSON reading and writing.
//!
//! The parser intentionally accepts a strict subset of JSON suited to the
//! frozen command grammar (SEC-001): maximum `MAX_REQUEST_BYTES` input,
//! nesting depth 3 (root object → params object/array → scalar), no
//! exponent numbers, no `\u` escapes, no duplicate keys, no trailing
//! tokens. Unsupported forms are explicit errors that cause no mutation.
//! The stratified `Value`/`Item`/`Leaf` types keep the tree fixed-size
//! without `alloc`. The writer is hand-rolled so output bytes are
//! deterministic and fixture-comparable.

use crate::MAX_REQUEST_BYTES;
use heapless::Vec;

/// Maximum object entries per object.
pub const MAX_OBJECT_ENTRIES: usize = 12;
/// Maximum array items per array.
pub const MAX_ARRAY_ITEMS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrChar {
    pub index: usize,
    pub byte: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JsonError {
    TooLarge,
    Eof,
    Unexpected(ErrChar),
    Depth,
    TooManyEntries,
    DuplicateKey,
    BadNumber,
    BadEscape,
    ControlChar,
    Trailing,
}

/// Top level of a parsed document: a flat object (the command envelope).
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Object(Vec<(&'a str, Item<'a>), MAX_OBJECT_ENTRIES>),
}

/// Second level: a scalar, an array of scalars, or an object of scalars.
#[derive(Clone, Debug, PartialEq)]
pub enum Item<'a> {
    Scalar(Leaf<'a>),
    Array(Vec<Leaf<'a>, MAX_ARRAY_ITEMS>),
    Object(Vec<(&'a str, Leaf<'a>), MAX_OBJECT_ENTRIES>),
}

/// Third level: scalars only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Leaf<'a> {
    Null,
    Bool(bool),
    Num(f64),
    Str(&'a str),
}

impl<'a> Value<'a> {
    pub fn get(&self, key: &str) -> Option<&Item<'a>> {
        match self {
            Value::Object(entries) => entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v),
        }
    }
}

impl<'a> Item<'a> {
    pub fn get(&self, key: &str) -> Option<&Leaf<'a>> {
        match self {
            Item::Object(entries) => entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Item::Scalar(Leaf::Num(n)) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Item::Scalar(Leaf::Str(s)) => Some(s),
            _ => None,
        }
    }
}

impl<'a> Leaf<'a> {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Leaf::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Leaf::Str(s) => Some(s),
            _ => None,
        }
    }
}

pub fn parse(input: &[u8]) -> Result<Value<'_>, JsonError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(JsonError::TooLarge);
    }
    let mut p = Parser { input, pos: 0 };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.pos != input.len() {
        return Err(JsonError::Trailing);
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Result<u8, JsonError> {
        self.input.get(self.pos).copied().ok_or(JsonError::Eof)
    }

    fn bump(&mut self) -> Result<u8, JsonError> {
        let b = self.peek()?;
        self.pos += 1;
        Ok(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.input.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn err_at(&self, kind: fn(ErrChar) -> JsonError) -> JsonError {
        kind(ErrChar {
            index: self.pos,
            byte: self.input.get(self.pos).copied().unwrap_or(0),
        })
    }

    fn literal(&mut self, word: &[u8]) -> Result<(), JsonError> {
        if self.input.len() - self.pos < word.len()
            || &self.input[self.pos..self.pos + word.len()] != word
        {
            return Err(self.err_at(JsonError::Unexpected));
        }
        self.pos += word.len();
        Ok(())
    }

    /// Depth 1: the root must be an object (command envelope).
    fn value(&mut self) -> Result<Value<'a>, JsonError> {
        if self.peek()? != b'{' {
            return Err(self.err_at(JsonError::Unexpected));
        }
        self.bump()?;
        let mut entries: Vec<(&'a str, Item<'a>), MAX_OBJECT_ENTRIES> = Vec::new();
        self.skip_ws();
        if self.peek()? == b'}' {
            self.bump()?;
            return Ok(Value::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek()? != b'"' {
                return Err(self.err_at(JsonError::Unexpected));
            }
            let key = self.string()?;
            if entries.iter().any(|(k, _)| *k == key) {
                return Err(JsonError::DuplicateKey);
            }
            self.skip_ws();
            if self.bump()? != b':' {
                return Err(self.err_at(JsonError::Unexpected));
            }
            self.skip_ws();
            let item = self.item()?;
            if entries.push((key, item)).is_err() {
                return Err(JsonError::TooManyEntries);
            }
            self.skip_ws();
            match self.bump()? {
                b',' => {}
                b'}' => return Ok(Value::Object(entries)),
                _ => return Err(self.err_at(JsonError::Unexpected)),
            }
        }
    }

    /// Depth 2: scalar, array of scalars, or object of scalars. Arrays of
    /// objects are rejected (depth limit) rather than silently accepted.
    fn item(&mut self) -> Result<Item<'a>, JsonError> {
        match self.peek()? {
            b'{' => {
                self.bump()?;
                let mut entries: Vec<(&'a str, Leaf<'a>), MAX_OBJECT_ENTRIES> = Vec::new();
                self.skip_ws();
                if self.peek()? == b'}' {
                    self.bump()?;
                    return Ok(Item::Object(entries));
                }
                loop {
                    self.skip_ws();
                    if self.peek()? != b'"' {
                        return Err(self.err_at(JsonError::Unexpected));
                    }
                    let key = self.string()?;
                    if entries.iter().any(|(k, _)| *k == key) {
                        return Err(JsonError::DuplicateKey);
                    }
                    self.skip_ws();
                    if self.bump()? != b':' {
                        return Err(self.err_at(JsonError::Unexpected));
                    }
                    self.skip_ws();
                    let leaf = self.leaf()?;
                    if entries.push((key, leaf)).is_err() {
                        return Err(JsonError::TooManyEntries);
                    }
                    self.skip_ws();
                    match self.bump()? {
                        b',' => {}
                        b'}' => return Ok(Item::Object(entries)),
                        _ => return Err(self.err_at(JsonError::Unexpected)),
                    }
                }
            }
            b'[' => {
                self.bump()?;
                let mut items: Vec<Leaf<'a>, MAX_ARRAY_ITEMS> = Vec::new();
                self.skip_ws();
                if self.peek()? == b']' {
                    self.bump()?;
                    return Ok(Item::Array(items));
                }
                loop {
                    self.skip_ws();
                    let leaf = self.leaf()?;
                    if items.push(leaf).is_err() {
                        return Err(JsonError::TooManyEntries);
                    }
                    self.skip_ws();
                    match self.bump()? {
                        b',' => {}
                        b']' => return Ok(Item::Array(items)),
                        _ => return Err(self.err_at(JsonError::Unexpected)),
                    }
                }
            }
            _ => Ok(Item::Scalar(self.leaf()?)),
        }
    }

    /// Depth 3: scalar leaf.
    fn leaf(&mut self) -> Result<Leaf<'a>, JsonError> {
        match self.peek()? {
            b'"' => Ok(Leaf::Str(self.string()?)),
            b'{' | b'[' => Err(JsonError::Depth),
            b't' => {
                self.literal(b"true")?;
                Ok(Leaf::Bool(true))
            }
            b'f' => {
                self.literal(b"false")?;
                Ok(Leaf::Bool(false))
            }
            b'n' => {
                self.literal(b"null")?;
                Ok(Leaf::Null)
            }
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.err_at(JsonError::Unexpected)),
        }
    }

    /// Parse a string and return the raw slice between quotes. Only escapes
    /// that do not alter the byte content (`\"`, `\\`, `\/`) are accepted;
    /// everything else is a bounded-grammar rejection so command payloads
    /// always map to exact input bytes.
    fn string(&mut self) -> Result<&'a str, JsonError> {
        self.bump()?; // '"'
        let start = self.pos;
        loop {
            let b = self.bump()?;
            match b {
                b'"' => {
                    let raw = &self.input[start..self.pos - 1];
                    return core::str::from_utf8(raw)
                        .map_err(|_| self.err_at(JsonError::Unexpected));
                }
                b'\\' => {
                    let esc = self.bump()?;
                    if !matches!(esc, b'"' | b'\\' | b'/') {
                        return Err(JsonError::BadEscape);
                    }
                }
                0x00..=0x1f => return Err(JsonError::ControlChar),
                _ => {}
            }
        }
    }

    /// Bounded number grammar: optional minus, integer digits with no
    /// redundant leading zeros, optional fractional digits. Exponents and
    /// `NaN`/`Infinity` are rejected.
    fn number(&mut self) -> Result<Leaf<'a>, JsonError> {
        let start = self.pos;
        let negative = self.peek()? == b'-';
        if negative {
            self.bump()?;
        }
        match self.peek()? {
            b'0' => {
                self.bump()?;
                if self.peek()?.is_ascii_digit() {
                    return Err(JsonError::BadNumber);
                }
            }
            b'1'..=b'9' => {
                while matches!(self.peek(), Ok(b'0'..=b'9')) {
                    self.bump()?;
                }
            }
            _ => return Err(JsonError::BadNumber),
        }
        let mut mantissa: f64 = 0.0;
        for b in &self.input[start + if negative { 1 } else { 0 }..self.pos] {
            mantissa = mantissa * 10.0 + f64::from(b - b'0');
        }
        if self.peek()? == b'.' {
            self.bump()?;
            let mut scale = 1.0;
            let mut seen = false;
            while matches!(self.peek(), Ok(b'0'..=b'9')) {
                let b = self.bump()?;
                scale *= 10.0;
                mantissa += f64::from(b - b'0') / scale;
                seen = true;
            }
            if !seen {
                return Err(JsonError::BadNumber);
            }
        }
        if self.peek().map(|b| b == b'e' || b == b'E').unwrap_or(false) {
            return Err(JsonError::BadNumber);
        }
        if negative {
            mantissa = -mantissa;
        }
        if !mantissa.is_finite() {
            return Err(JsonError::BadNumber);
        }
        Ok(Leaf::Num(mantissa))
    }
}

// ------------------------------------------------------------------ writer --

pub fn write_str<W: core::fmt::Write>(out: &mut W, value: &str) -> core::fmt::Result {
    out.write_str("\"")?;
    for c in value.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if (c as u32) < 0x20 => {
                // Control characters cannot appear in validated payloads;
                // reject rather than emit invalid JSON.
                return Err(core::fmt::Error);
            }
            c => {
                let mut buf = [0u8; 4];
                out.write_str(c.encode_utf8(&mut buf))?;
            }
        }
    }
    out.write_str("\"")
}

pub fn write_i64<W: core::fmt::Write>(out: &mut W, value: i64) -> core::fmt::Result {
    write!(out, "{value}")
}

pub fn write_u64<W: core::fmt::Write>(out: &mut W, value: u64) -> core::fmt::Result {
    write!(out, "{value}")
}

pub fn write_bool<W: core::fmt::Write>(out: &mut W, value: bool) -> core::fmt::Result {
    out.write_str(if value { "true" } else { "false" })
}

/// Round to a fixed number of decimals and emit a decimal without
/// exponents, e.g. `812.0`, `-12.34`.
pub fn write_fixed<W: core::fmt::Write>(
    out: &mut W,
    value: f64,
    decimals: u32,
) -> core::fmt::Result {
    if !value.is_finite() {
        return Err(core::fmt::Error);
    }
    let mut factor = 1i64;
    for _ in 0..decimals {
        factor *= 10;
    }
    let scaled = if value < 0.0 {
        (value * factor as f64 - 0.5) as i64
    } else {
        (value * factor as f64 + 0.5) as i64
    };
    let negative = scaled < 0;
    let magnitude = scaled.unsigned_abs();
    if decimals == 0 {
        if negative {
            out.write_str("-")?;
        }
        return write_u64(out, magnitude);
    }
    let whole = magnitude / factor.unsigned_abs();
    let frac = magnitude % factor.unsigned_abs();
    if negative {
        out.write_str("-")?;
    }
    write_u64(out, whole)?;
    out.write_str(".")?;
    // Zero-pad fractional digits to `decimals` width.
    let mut pad = 1u64;
    for _ in 1..decimals {
        pad *= 10;
    }
    while pad > 0 {
        let digit = (frac / pad) % 10;
        let mut buf = [0u8; 1];
        buf[0] = b'0' + digit as u8;
        out.write_str(core::str::from_utf8(&buf).map_err(|_| core::fmt::Error)?)?;
        pad /= 10;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Item, JsonError, Leaf, Value, parse};

    #[test]
    fn parses_command_envelope() {
        let input = b"{\"request_id\":\"r1\",\"op\":\"snapshot\",\"params\":{\"limit\":10}}";
        let value = parse(input).unwrap();
        let Value::Object(entries) = &value;
        assert_eq!(entries.len(), 3);
        assert_eq!(value.get("request_id").and_then(Item::as_str), Some("r1"));
        assert_eq!(
            value
                .get("params")
                .and_then(|p| p.get("limit"))
                .and_then(Leaf::as_f64),
            Some(10.0)
        );
    }

    #[test]
    fn rejects_depth_beyond_grammar() {
        // Nested object inside params violates the depth-3 rule.
        let input = b"{\"params\":{\"inner\":{\"deep\":1}}}";
        assert_eq!(parse(input).unwrap_err(), JsonError::Depth);
    }

    #[test]
    fn rejects_duplicate_keys_and_trailing() {
        assert_eq!(
            parse(b"{\"a\":1,\"a\":2}").unwrap_err(),
            JsonError::DuplicateKey
        );
        assert!(matches!(
            parse(b"{\"a\":1} x").unwrap_err(),
            JsonError::Trailing
        ));
    }

    #[test]
    fn number_grammar_is_bounded() {
        assert!(matches!(
            parse(b"{\"a\":01}").unwrap_err(),
            JsonError::BadNumber
        ));
        assert!(matches!(
            parse(b"{\"a\":1e5}").unwrap_err(),
            JsonError::BadNumber
        ));
        assert!(matches!(
            parse(b"{\"a\":1.}").unwrap_err(),
            JsonError::BadNumber
        ));
    }

    #[test]
    fn negative_and_fractional_numbers_parse() {
        let v = parse(b"{\"a\":-2.5}").unwrap();
        assert_eq!(v.get("a").and_then(Item::as_f64), Some(-2.5));
    }

    #[test]
    fn arrays_of_scalars_parse() {
        let v = parse(b"{\"faults\":[\"a\",\"b\"]}").unwrap();
        match v.get("faults") {
            Some(Item::Array(items)) => assert_eq!(items.len(), 2),
            other => panic!("expected array, got {other:?}"),
        }
    }
}
