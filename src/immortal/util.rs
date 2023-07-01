
use std::str;
use std::str::Utf8Error;
use std::collections::HashMap;

use super::Response;

/// accepts a response and returns true if it is a redirect response
pub fn is_redirect(response: &Response) -> bool {
    let mut cases = 0;
    if response.code.starts_with('3') {
        cases += 1;
    }
    if response.header("Location").is_some() {
        cases += 1;
    }
    cases == 2
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
pub fn url_decode(to_decode: &str) -> Result<String, Utf8Error> {
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
    Ok(str::from_utf8(&build)?.to_string())
}

/// Parses an HTTP query string into a key-value hashmap
pub fn parse_parameters(to_parse: &str) -> HashMap<String, String> {
    if to_parse.is_empty() {
        return HashMap::new();
    }

    #[derive(Debug, PartialEq, Eq)]
    enum ParseState {
        Name,
        Value,
    }
    let mut state = ParseState::Name;

    let mut params = HashMap::new();
    let mut name = String::new();
    let mut value;
    let mut builder = String::new();

    // perform state machine parsing on the query string
    for c in to_parse.chars() {
        match c {
            // transition to value parsing
            '=' => {
                if state == ParseState::Value {
                    builder.push(c);
                    continue;
                }
                name = builder.clone();
                if name.is_empty() {
                    name = String::new();
                } else {
                    builder = String::new();
                    state = ParseState::Value;
                }
            },
            // transition to name parsing
            '&' => {
                if !name.is_empty() {
                    value = builder;
                    if value.is_empty() {
                        builder = String::new();
                    } else {
                        builder = String::new();
                        if is_param_name_valid(&name) {
                            value = match url_decode(&value) {
                                Err(_) => continue,
                                Ok(v) => v,
                            };
                            params.insert(name.clone(), value.clone());
                        }
                        state = ParseState::Name;
                    }
                }
            },
            _ => {
                builder.push(c);
            },
        };
    }

    if state == ParseState::Value && !name.is_empty() {
        value = builder;
        if !value.is_empty() && is_param_name_valid(&name) {
            value = match url_decode(&value) {
                Err(_) => return params,
                Ok(v) => v,
            };
            params.insert(name, value);
        }
    }

    params
}

/// Accepts a slice containing unparsed headers straight from the request recieve buffer, split and
/// parse these into a hashmap of key-value pairs where keys have all ascii values as uppercase.
pub fn parse_headers(to_parse: &[u8]) -> Result<HashMap<String, String>, Utf8Error> {
    let to_parse = str::from_utf8(to_parse)?;
    //let to_parse = match str::from_utf8(to_parse) {
    //    Err(e) => return Err(Error::new(ErrorKind::InvalidData, format!("{}", e))),
    //    Ok(s) => s.trim_end_matches(|c| c == '\0'), // could be the rest of the recv buffer if
    //                                                // we're not careful
    //};

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
            Some(thing) => {
                // if the header value is empty or the header is invalid, skip the header
                // is_empty is faster O(1) than is_param_name_valid O(n),
                //  so short circuit the is_empty call first
                if thing.1.is_empty() || !is_param_name_valid(thing.0) {
                    continue;
                } else {
                    // gets the strings as copies, makes the key uppercase for case insensitivity
                    (thing.0.to_ascii_uppercase(), thing.1.to_string())
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

/// Find the index of the first item `by`, and return a tuple of two mutable string slices, the
/// first being the slice content up to the first instance of item `by`, and the second being the
/// slice content after the first instance of `by`.
/// 
/// This exists because there is no split_once in a slice, only for strings
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

