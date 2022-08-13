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

#[derive(Debug)]
pub struct Request {
    body: Vec<u8>,
    method: String,
    document: String,
    query: String,
    protocol: String,
    version: String,
    //headers: Vec<&mut [u8]>,
    
    //Future header fields to be parsed
    //host: String
    //user_agent: String,
    //connection: String,
    //content_type: String,
    //content_length: String,
    //cookies_raw: String,
    //keep_alive: bool,
}

impl Request {
    /**
     * Construct a new request object, parsing the request buffer
     */
    pub fn new(buf: &mut [u8]) -> Result<Self, String> {
        let (request_head, request_body) = match request_head_body_split(buf) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };
        let body = match request_body {
            None => vec![],
            Some(thing) => thing.to_vec(),
        };

        let (request_line, request_headers) = match request_line_header_split(request_head) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };

        let request_line_items: [&[u8]; 3] = match request_line
            .split(|c| *c == b' ')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err("Invalid request line".to_string()),
                Ok(items) => items,
        };

        let method = match str::from_utf8(&request_line_items[0].to_ascii_uppercase()) {
            Err(e) => return Err(format!("Invalid method string: {}", e)),
            Ok(m) => m.to_string(),
        };

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
        
        let proto_version_items: [&[u8]; 2] = match request_line_items[2]
            .split(|c| *c == b'/')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err("Invalid proto string".to_string()),
                Ok(items) => items,
        };
        let protocol = match str::from_utf8(proto_version_items[0]) {
            Err(e) => return Err(format!("Invalid protocol string: {}", e)),
            Ok(p) => p.to_string(),
        };
        if protocol != "HTTP" {
            return Err("Invalid protocol in proto string".to_string());
        }

        let version = match str::from_utf8(proto_version_items[1]) {
            Err(e) => return Err(format!("Invalid version string: {}", e)),
            Ok(v) => v.trim_end_matches(|c| c == '\r' || c == '\n' || c == '\0').to_string(),
        };

        if version != "1.1" {
            return Err("Invalid version in proto string".to_string());
        }

        Ok(Self {
            body,
            method,
            document,
            query,
            protocol,
            version,
        })
    }
}

/**
 * Find the index of the first item `by`, and return a tuple of two mutable string slices, the
 * first being the slice content up to the first instance of item `by`, and the second being the
 * slice content after the first instance of `by`.
 *
 * This exists because there is no split_once in a mutable slice, only for strings
 */
fn split_once(to_split: &[u8], by: u8) -> Result<(&[u8], Option<&[u8]>), String> {
    let mut found_idx = 0;

    for (idx, byte) in to_split.iter().enumerate() {
        if *byte == by {
            found_idx = idx;
            break;
        }
    }

    if found_idx == 0 {
        return Ok((to_split, None));
    }

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

    if crlf_start_idx == 0 {
        let line_cleaned = match to_split.strip_suffix(&[b'\r', b'\n']) {
            None => return Ok((to_split, None)),
            Some(thing) => thing,
        };
        return Ok((line_cleaned, None));
    }

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

    if crlf_start_idx == 0 {
        return Ok((to_split, None));
    }

    let (head, body) = to_split.split_at(crlf_start_idx);
    let body = body.split_at(4).1;
    Ok((head, Some(body)))
}

