/**
*     Copyright (C) 2022 Mason Soroka-Gill
*
*     This program is free software: you can redistribute it and/or modify
*     it under the terms of the GNU General Public License as published by
*     the Free Software Foundation, either version 3 of the License, or
*     (at your option) any later version.
*
*     This program is distributed in the hope that it will be useful,
*     but WITHOUT ANY WARRANTY; without even the implied warranty of
*     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*     GNU General Public License for more details.
*
*     You should have received a copy of the GNU General Public License
*     along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::str;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Request {
    pub body: Vec<u8>,
    pub method: String,
    pub document: String,
    pub query: String,
    pub protocol: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub get: HashMap<String, String>,
    
    pub host: String,
    pub user_agent: String,
    pub connection: String,
    pub content_type: String,
    pub content_length: usize,
    pub keep_alive: bool,
}

impl Request {
    /**
     * Construct a new request object, parsing the request buffer
     */
    pub fn new(buf: &mut [u8]) -> Result<Self, String> {
        // parse body
        let (request_head, request_body) = match request_head_body_split(buf) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };
        let body = match request_body {
            None => vec![],
            Some(thing) => thing.to_vec(),
        };

        // split request line and header chunk
        let (request_line, request_headers) = match request_line_header_split(request_head) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };

        // split to each major component of a request line
        let request_line_items: [&[u8]; 3] = match request_line
            .split(|c| *c == b' ')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err("Invalid request line".to_string()),
                Ok(items) => items,
        };

        // parse method
        let method = match str::from_utf8(&request_line_items[0].to_ascii_uppercase()) {
            Err(e) => return Err(format!("Invalid method string: {}", e)),
            Ok(m) => m.to_string(),
        };

        // parse document and query
        let (document, query) = match split_once(request_line_items[1], b'?') {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };
        let document = match str::from_utf8(document) {
            Err(e) => return Err(format!("Invalid document string: {}", e)),
            Ok(d) => d.to_string(),
        };
        let query = match query {
            None => "".to_string(),
            Some(thing) => match str::from_utf8(thing) {
                Err(e) => return Err(format!("Invalid query string: {}", e)),
                Ok(q) => q.to_string(),
            },
        };
        
        // parse protocol/version components
        let proto_version_items: [&[u8]; 2] = match request_line_items[2]
            .split(|c| *c == b'/')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err("Invalid proto string".to_string()),
                Ok(items) => items,
        };

        // parse protocol string
        let protocol = match str::from_utf8(proto_version_items[0]) {
            Err(e) => return Err(format!("Invalid protocol string: {}", e)),
            Ok(p) => p.to_string(),
        };
        if protocol != "HTTP" {
            return Err("Invalid protocol in proto string".to_string());
        }

        // parse protocol version string
        let version = match str::from_utf8(proto_version_items[1]) {
            Err(e) => return Err(format!("Invalid version string: {}", e)),
            Ok(v) => v.trim_end_matches(|c| c == '\r' || c == '\n' || c == '\0').to_string(),
        };
        if version != "1.1" {
            return Err("Invalid version in proto string".to_string());
        }

        // parse headers
        let headers = match parse_headers(match request_headers {
            None => b"",
            Some(thing) => thing,
        }) {
            Err(e) => return Err(format!("Invalid header string: {}", e)),
            Ok(h) => h,
        };

        // collect common headers from the `headers` HashMap
        let host = collect_header(&headers, "Host");
        let user_agent = collect_header(&headers, "User-Agent");
        let connection = collect_header(&headers, "Connection");
        let content_type = collect_header(&headers, "Content-Type");
        let content_length = collect_header(&headers, "Content-Length");

        // parse keep-alive status as bool
        let keep_alive = connection == "keep-alive";

        // parse `content_length` else return the index of the first null char in the body
        let content_length = match content_length.parse::<usize>() {
            Err(_) => body
                .iter()
                .take_while(|c| **c != b'\0')
                .count(),
            Ok(l) => l,
        };

        let get = match parse_parameters(&query) {
            Err(_) => HashMap::new(),
            Ok(g) => g,
        };

        // emit a complete Request object
        Ok(Self {
            body,
            method,
            document,
            query,
            protocol,
            version,
            headers,
            get,
            host,
            user_agent,
            connection,
            content_type,
            content_length,
            keep_alive,
        })
    }

    /**
     * Accepts a string reference and looks it up in the `self.headers` hashmap
     *  if the string is there, return a mutable reference to its value
     *  if not, emplace an empty string into the hashmap and return it as a mutable reference
     *
     *  Should be used like the following:
     *    Read:
     *      let my = self.header("Content-Type").unwrap();
     *    Write:
     *      self.header("Content-Type").unwrap() = "application/x-www-form-urlencoded";
     */
    pub fn header(&mut self, key: &str) -> Option<&mut str> {
        if key.is_empty() || !is_param_name_valid(key) { return None }
        let key = key.to_ascii_uppercase();
        let key_copy = key.to_string();

        if self.headers.contains_key(&key_copy) {
            return Some(self.headers.get_mut(&key_copy).unwrap());
        } else {
            self.headers.insert(key, String::new());
            return Some(self.headers.get_mut(&key_copy).unwrap());
        }
    }
}

/**
 * Accepts a hashmap and a key, returns the key value or an empty string
 * [[[ASSUMES `key` IS A VALID NON EMPTY HEADER KEY]]]: handling errors from this will be annoying.
 */
fn collect_header(headers: &HashMap<String, String>, key: &str) -> String {
    let key = key.to_ascii_uppercase();
    match headers.get(&key) {
        None => String::new(),
        Some(thing) => thing.to_string(),
    }
}

/**
 * Accept a byte encoding a hex value and decompose it into its decimal form
 */
fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'0'..=b'9' => Some(byte - b'0'),
        _ => None,
    }
}

/**
 * Accept a string, perform URL decoding on the string and return the result
 */
fn url_decode(to_decode: &str) -> Result<String, String> {
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
            other => build.push(other),
        }
    }

    // validate if is still utf8
    match String::from_utf8(build) {
        Err(e) => Err(format!("{}", e)),
        Ok(decoded) => Ok(decoded),
    }
}

fn parse_parameters(to_parse: &str) -> Result<HashMap<String, String>, String> {
    if to_parse.is_empty() {
        return Ok(HashMap::new());
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
                Err(_) => return Ok(params),
                Ok(v) => v,
            };
            params.insert(name, value);
        }
    }

    Ok(params)
}

/**
 * Accepts a slice containing unparsed headers straight from the request recieve buffer, split and
 * parse these into a hashmap of key-value pairs where keys have all ascii values as uppercase.
 */
fn parse_headers(to_parse: &[u8]) -> Result<HashMap<String, String>, String> {
    let to_parse = match str::from_utf8(to_parse) {
        Err(e) => return Err(format!("{}", e)),
        Ok(s) => s.trim_end_matches(|c| c == '\0'), // could be the rest of the recv buffer if
                                                    // we're not careful
    };

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
        let (header_key, header_value) = match raw_header.split_once(&": ") {
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

/**
 * Accepts a string which is assumed to be a param name.
 * Returns true if it's valid, valse if it's not valid.
 */
fn is_param_name_valid(param: &str) -> bool {
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

/**
 * Find the index of the first item `by`, and return a tuple of two mutable string slices, the
 * first being the slice content up to the first instance of item `by`, and the second being the
 * slice content after the first instance of `by`.
 *
 * This exists because there is no split_once in a slice, only for strings
 */
fn split_once(to_split: &[u8], by: u8) -> Result<(&[u8], Option<&[u8]>), String> {
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
        return Ok((to_split, None));
    }

    // build the returned tuple excluding the matched `by` in the data
    let (item, rest) = to_split.split_at(found_idx);
    let rest = rest.split_at(1).1;
    Ok((item, Some(rest)))
}

/**
 * Find the index of the first crlf and return a tuple of two mutable string slices, the first
 * being the buffer slice up to the crlf, and the second being the slice content after the
 * clrf
 */
fn request_line_header_split(to_split: &[u8]) -> Result<(&[u8], Option<&[u8]>), String> {
    let mut found_cr = false;
    let mut crlf_start_idx = 0;

    // iterate over the slice and get the index of the first crlf
    for (idx, byte) in to_split.iter().enumerate() {
        if *byte == b'\r' {
            crlf_start_idx = idx;
            found_cr = true;
            continue;
        }
        if found_cr && *byte == b'\n' {
            break;
        }
        crlf_start_idx = 0;
        found_cr = false;
    }

    // if no crlf was found or its at index 0, strip off crlf if possible and then return it
    if crlf_start_idx == 0 {
        let line_cleaned = match to_split.strip_suffix(&[b'\r', b'\n']) {
            None => return Ok((to_split, None)),
            Some(thing) => thing,
        };
        return Ok((line_cleaned, None));
    }

    // build the returned tuple excluding the crlf in the data
    let (req_line, req_headers) = to_split.split_at(crlf_start_idx);
    let req_headers = req_headers.split_at(2).1;
    Ok((req_line, Some(req_headers)))
}

/**
 * Find the index of the first double crlf and return a tuple of two mutable string slices, the
 * first being the slice content up to the double crlf, and the second being being the slice 
 * content after the double clrf
 */
fn request_head_body_split(to_split: &[u8]) -> Result<(&[u8], Option<&[u8]>), String>  {
    let mut found_cr = false;
    let mut crlf_count = 0;
    let mut crlf_start_idx = 0;

    // iterate over the slice and get the index of the first double crlf
    for (idx, byte) in to_split.iter().enumerate() {
        if crlf_count == 2 { // exit case where crlf_start_index can be not zero
            break;
        }
        if *byte == b'\r' {
            if crlf_count == 0 { // record the crlf start index only on the first crlf
                crlf_start_idx = idx;
            }
            found_cr = true;
            continue;
        }
        if found_cr && *byte == b'\n' {
            crlf_count += 1;
            found_cr = false;
            continue;
        }
        crlf_count = 0;
        crlf_start_idx = 0;
        found_cr = false;
    }

    // if no double crlf was found or its index is at 0, return it
    if crlf_start_idx == 0 {
        return Ok((to_split, None));
    }

    // build the returned tuple excluding the double crlf in the data
    let (head, body) = to_split.split_at(crlf_start_idx);
    let body = body.split_at(4).1;
    Ok((head, Some(body)))
}

