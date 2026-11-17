// Python's text rules, for messages and orderings that follow them.

use std::fs;
use std::io;
use std::path::Path;
use std::str::{self, Utf8Error};

fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

pub fn rstrip(text: &str) -> &str {
    text.trim_end_matches(is_space)
}

// Lines with their endings, split where `is_break` holds; "\r\n" is one ending.
pub fn split_at_breaks(text: &str, is_break: fn(char) -> bool) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if is_break(c) {
            let mut end = at + c.len_utf8();
            if c == '\r' {
                if let Some(&(next, '\n')) = chars.peek() {
                    chars.next();
                    end = next + 1;
                }
            }
            lines.push(&text[start..end]);
            start = end;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

fn without_ending(line: &str, is_break: fn(char) -> bool) -> &str {
    if let Some(rest) = line.strip_suffix("\r\n") {
        return rest;
    }
    match line.chars().next_back() {
        Some(c) if is_break(c) => &line[..line.len() - c.len_utf8()],
        _ => line,
    }
}

fn is_newline(c: char) -> bool {
    c == '\n' || c == '\r'
}

// The lines a text file yields in Python's universal newline mode, without their endings.
pub fn file_lines(text: &str) -> Vec<&str> {
    split_at_breaks(text, is_newline)
        .into_iter()
        .map(|line| without_ending(line, is_newline))
        .collect()
}

fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

// str.splitlines(), which also breaks on vertical tab, form feed and the other separators.
pub fn splitlines(text: &str) -> Vec<&str> {
    split_at_breaks(text, is_line_break)
        .into_iter()
        .map(|line| without_ending(line, is_line_break))
        .collect()
}

fn is_printable(c: char) -> bool {
    if c.is_ascii() {
        return (' '..='~').contains(&c);
    }
    let code = c as u32;
    !(c.is_whitespace()
        || c.is_control()
        || matches!(code, 0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x206F | 0xE000..=0xF8FF | 0xFEFF | 0xFFF9..=0xFFFB))
}

// Python's repr() of a str. Printability of non-ASCII characters is approximated.
pub fn repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c if (c as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push(quote);
    out
}

// str(OSError) for an error about one file: "[Errno 2] No such file or directory: 'x'".
pub fn os_error(error: &io::Error, filename: &str) -> String {
    os_error_text(error, &repr(filename))
}

// str(OSError) with the filenames already shown, as in "'a' -> 'b'" for a rename.
pub fn os_error_text(error: &io::Error, filenames: &str) -> String {
    let text = error.to_string();
    match error.raw_os_error() {
        Some(code) => {
            let reason = text
                .strip_suffix(&format!(" (os error {code})"))
                .unwrap_or(&text);
            format!("[Errno {code}] {reason}: {filenames}")
        }
        None => text,
    }
}

// str(UnicodeDecodeError) for strict UTF-8 decoding of `bytes`.
pub fn decode_error(bytes: &[u8], error: &Utf8Error) -> String {
    let start = error.valid_up_to();
    let (count, reason) = match error.error_len() {
        Some(count) => {
            let first = bytes[start];
            let starts_wrong = (0x80..=0xC1).contains(&first) || first >= 0xF5;
            (
                count,
                if starts_wrong {
                    "invalid start byte"
                } else {
                    "invalid continuation byte"
                },
            )
        }
        None => (bytes.len() - start, "unexpected end of data"),
    };
    let span = if count == 1 {
        format!("byte 0x{:02x} in position {start}", bytes[start])
    } else {
        format!("bytes in position {start}-{}", start + count - 1)
    };
    format!("'utf-8' codec can't decode {span}: {reason}")
}

// The code points Python sees for a file name: undecodable bytes become U+DC80 to U+DCFF.
pub fn name_points(bytes: &[u8]) -> Vec<u32> {
    let mut points = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match str::from_utf8(rest) {
            Ok(text) => {
                points.extend(text.chars().map(|c| c as u32));
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                points.extend(
                    str::from_utf8(&rest[..valid])
                        .unwrap()
                        .chars()
                        .map(|c| c as u32),
                );
                let invalid = error.error_len().unwrap_or(rest.len() - valid);
                points.extend(
                    rest[valid..valid + invalid]
                        .iter()
                        .map(|b| 0xDC00 + *b as u32),
                );
                rest = &rest[valid + invalid..];
            }
        }
    }
    points
}

// A file name with undecodable bytes as \udcXX, as Python's backslashreplace prints it.
pub fn escaped(bytes: &[u8]) -> String {
    name_points(bytes)
        .into_iter()
        .map(|point| match point {
            0xDC80..=0xDCFF => format!("\\udc{:02x}", point & 0xFF),
            _ => char::from_u32(point).unwrap().to_string(),
        })
        .collect()
}

pub enum ReadError {
    Missing,
    Io(io::Error),
    Decode(String),
}

// A file read as strict UTF-8, as Python's open() reads it.
pub fn read_utf8(path: &Path) -> Result<String, ReadError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(ReadError::Missing),
        Err(error) => return Err(ReadError::Io(error)),
    };
    String::from_utf8(bytes)
        .map_err(|error| ReadError::Decode(decode_error(error.as_bytes(), &error.utf8_error())))
}

impl ReadError {
    // "{prefix} missing", or "{prefix}: cannot read: {reason}" where
    // the OS reason names the file as `filename`.
    pub fn describe(&self, prefix: &str, filename: &str) -> String {
        match self {
            ReadError::Missing => format!("{prefix} missing"),
            ReadError::Io(error) => format!("{prefix}: cannot read: {}", os_error(error, filename)),
            ReadError::Decode(reason) => format!("{prefix}: cannot read: {reason}"),
        }
    }
}
