
use std::collections::HashMap;
use std::fmt::Display;
use std::str::{self, Utf8Error, Chars};
use std::error;

use colored::{Colorize, ColoredString};

#[derive(Debug)]
pub enum ParseError {
    ParamNameInvalid(String),
    UrlDecodeNotUtf8(Utf8Error),
    MalformedParams(String, String),
}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for ParseError {}

/// colours an HTTP code appropriately
pub fn code_color(code: &str) -> ColoredString {
    match code.as_bytes().first() {
        Some(n) => match n {
            b'1' => code.white().bold(),
            b'2' => code.green(),
            b'3' => code.cyan().bold(),
            b'4' => code.yellow(),
            b'5' => code.red().bold(),
            _ => code.normal(),
        },
        None => {
            "<no response code>".red().bold()
        },
    }
}

/// Performs html escaping on str
pub fn escape_html(str: &str) -> String {
    let mut out = String::new();
    for ch in str.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            ';' => out.push_str("&#59;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Accept a string, filter out the terminal control chars and return the clean string
pub fn strip_for_terminal(to_strip: &str) -> String {
    to_strip.chars()
        .filter(|chr| !matches!(chr, '\x07'..='\x0D'))
        .collect::<String>()
}

/// Accept a byte encoding a hex value and decompose it into its half-byte binary form
fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'0'..=b'9' => Some(byte - b'0'),
        _ => None,
    }
}

/// Accept a string, perform URL decoding on the string and return the result
pub fn url_decode(to_decode: &str) -> Result<String, ParseError> {
    let mut build: Vec<u8> = Vec::with_capacity(to_decode.len());
    let mut bytes = to_decode.bytes();
    while let Some(c) = bytes.next() {
        match c {
            b'%' => { // if % is found, take the next 2 characters and try to hex-decoe them
                match bytes.next() {
                    None => build.push(b'%'),
                    Some(top) => match from_hex(top) {
                        None => {
                            build.push(b'%');
                            build.push(top);
                        },
                        Some(t) => match bytes.next() {
                            None => {
                                build.push(b'%');
                                build.push(top);
                            },
                            Some(bottom) => match from_hex(bottom) {
                                None => { // fail, emit as-is
                                    build.push(b'%');
                                    build.push(top);
                                    build.push(bottom);
                                },
                                Some(b) => {
                                    // pack the top and bottom half of the byte then add it
                                    build.push((t << 4) | b);
                                },
                            },
                        },
                    },
                };
            },
            b'+' => build.push(b' '),
            b'\0' => break,
            other => build.push(other),
        }
    }

    // validate if is still utf8
    Ok(str::from_utf8(&build)
        .map_err(ParseError::UrlDecodeNotUtf8)?.to_string())
}

const EOF_CHAR: char = '\0';
/// Parser for Key-Value values delimited by '='
pub(crate) struct KVParser<'buf> {
    len_remaining: usize,
    chars: Chars<'buf>,
}

impl<'buf> KVParser<'buf> {
    pub(crate) fn new(input: &'buf str) -> KVParser<'buf> {
        KVParser {
            len_remaining: input.len(),
            chars: input.chars(),
        }
    }

    pub(crate) fn as_str(&self) -> &'buf str {
        self.chars.as_str()
    }

    pub(crate) fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    pub(crate) fn pos_within_token(&self) -> usize {
        self.len_remaining - self.chars.as_str().len()
    }

    pub(crate) fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    pub(crate) fn advance(&mut self) -> Option<char> {
        Some(self.chars.next()?)
    }

    pub(crate) fn consume_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.first()) && !self.is_eof() {
            self.advance();
        }
    }

    /// Advance token parser for query KV parsing
    pub(crate) fn query_kv_pair(&mut self) -> Option<(&'buf str, &'buf str)> {
        if !self.chars.as_str().is_ascii() {
            return None;
        }
        let iter = self.chars.clone();
        let first_char = match self.advance() {
            Some(c) => c,
            None => return None,
        };

        fn is_id_start(c: char) -> bool {
            (c == '_' || c == '-' || c == '&')
                || ('a' <= c && c <= 'z')
                || ('A' <= c && c <= 'Z')
        }
        fn is_id_continue(c: char) -> bool {
            (c == '_' || c == '-' || c == '&')
                || ('a' <= c && c <= 'z')
                || ('A' <= c && c <= 'Z')
                || ('0' <= c && c <= '9')
        }

        // get key len
        if is_id_start(first_char) {
            self.consume_while(|c| is_id_continue(c) && c != '=');
        } else {
            return None;
        }
        let key_len = self.pos_within_token();

        match self.advance() {
            // skip =
            Some('=') => (),
            // anything else, just do an empty value key
            _ => {
                let key = &iter.as_str()[..key_len];
                return Some((key, ""));
            },
        }
        self.reset_pos_within_token();

        let first_char = match self.advance() {
            Some(c) => c,
            // if no more, just do an empty value key
            None => {
                let key = &iter.as_str()[..key_len];
                return Some((key, ""));
            },
        };

        if first_char != '&' {
            self.consume_while(|c| c != '&');
        } else {
            let key = &iter.as_str()[..key_len];
            return Some((key, ""));
        }
        let val_len = self.pos_within_token();
        self.advance();
        self.reset_pos_within_token();

        let iter_str = iter.as_str();
        let key = &iter_str[..key_len];
        let value = &iter_str[(key_len+1)..(key_len+1+val_len)];
        return Some((key, value));
    }

    /// Advance token parser for cookie KV parsing
    pub(crate) fn cookie_kv_pair(&mut self) -> Option<(&'buf str, &'buf str)> {
        if !self.chars.as_str().is_ascii() {
            return None;
        }
        let iter = self.chars.clone();
        let first_char = match self.advance() {
            Some(c) => c,
            None => return None,
        };

        fn is_id_start(c: char) -> bool {
            (c == '_')
                || ('a' <= c && c <= 'z')
                || ('A' <= c && c <= 'Z')
        }
        fn is_id_continue(c: char) -> bool {
            (c == '_' || c == '-')
                || ('a' <= c && c <= 'z')
                || ('A' <= c && c <= 'Z')
                || ('0' <= c && c <= '9')
        }

        if is_id_start(first_char) {
            self.consume_while(|c| is_id_continue(c) && c != '=');
        } else {
            return None;
        }
        let key_len = self.pos_within_token();

        match self.advance() {
            // skip =
            Some('=') => (),
            // anything else, just do an empty value key
            _ => {
                let key = &iter.as_str()[..key_len];
                return Some((key, ""));
            },
        }
        self.reset_pos_within_token();

        let first_char = match self.advance() {
            Some(c) => c,
            // if no more, just do an empty value key
            None => {
                let key = &iter.as_str()[..key_len];
                return Some((key, ""));
            },
        };

        if (first_char.is_ascii_alphanumeric() || first_char.is_ascii_punctuation()) && first_char != ';' {
            self.consume_while(|c| c != ';');
        } else {
            let key = &iter.as_str()[..key_len];
            return Some((key, ""));
        }
        let val_len = self.pos_within_token();
        self.advance();
        self.reset_pos_within_token();

        let iter_str = iter.as_str();
        let key = &iter_str[..key_len];
        let value = &iter_str[(key_len+1)..(key_len+1+val_len)];
        return Some((key, value));
    }
}

/// Parses an HTTP query string into a key-value hashmap
/// Note: Assumes there will be no whitespace characters.
pub fn parse_parameters<'buf>(to_parse: &'buf str) -> Result<HashMap<&'buf str, &'buf str>, ParseError> {
    if to_parse.is_empty() {
        return Ok(HashMap::new());
    }

    let mut pp = KVParser::new(to_parse);
    let mut params = HashMap::new();

    while let Some((key, value)) = pp.query_kv_pair() {
        params.insert(key, value);
    }

    return Ok(params);
}

/// Accepts a slice containing unparsed headers straight from the request recieve buffer, split and
/// parse these into a hashmap of key-value pairs where keys have all ascii values as uppercase.
pub fn parse_headers<'buf>(to_parse: &'buf [u8]) -> Result<HashMap<&'buf str, &'buf str>, Utf8Error> {
    let to_parse = str::from_utf8(to_parse)?;

    if to_parse.is_empty() {
        return Ok(HashMap::new());
    }

    // split by crlf
    let headers_vec: Vec<&str> = to_parse
        .split(&"\r\n")
        .collect();

    // create a hashmap and populate it with parsed header key-value pairs
    let mut headers = HashMap::new();
    for raw_header in headers_vec {
        let (header_key, header_value) = match raw_header.split_once(": ") {
            None => continue,
            Some((key, value)) => {
                // if the header value is empty or the header is invalid, skip the header
                if value.is_empty() || !is_param_name_valid(key) {
                    continue;
                } else {
                    // gets the strings
                    (key, value)
                }
            },
        };

        headers.insert(header_key, header_value);
    }

    Ok(headers)
}

/// Accepts a string which is assumed to be a param name.
/// Returns true if it's valid, valse if it's not valid.
pub fn is_param_name_valid(param: &str) -> bool {
    if param.is_empty() { return false }

    for (idx, chr) in param.chars().enumerate() {
        if idx == 0 { // first char can't be a number
            if let '0'..='9' = chr { return false }
        }
        match chr { // can only be alphanumeric and '-' | '_'
            'a'..='z' => continue,
            'A'..='Z' => continue,
            '0'..='9' => continue,
            '-' => continue,
            '_' => continue,
            _ => return false,
        }
    }
    true
}

/// Find the index of the first item `by`, and return a tuple of two mutable string slices 
///
/// The first being the slice content up to the first instance of item `by`, and the second being 
/// the slice content after the first instance of `by`.
/// 
/// This exists because there is no stable split_once for u8 slices
pub fn split_once(to_split: &[u8], by: u8) -> (&[u8], Option<&[u8]>) {
    let mut found_idx = 0;

    // iterate over the slice and and obtain the first instance of `by` in `to_split`
    for (idx, byte) in to_split.iter().enumerate() {
        if *byte == by {
            found_idx = idx;
            break;
        }
    }

    // if `by` wasn't found in `to_split` or its at index 0, just return it
    if found_idx == 0 {
        return (to_split, None);
    }

    // build the returned tuple excluding the matched `by` in the data
    let (item, rest) = to_split.split_at(found_idx);
    let rest = rest.split_at(1).1;
    if rest == b"" {
        (item, None)
    } else {
        (item, Some(rest))
    }
}

