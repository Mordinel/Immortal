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
    method: String,
    document: String,
    query: String,
    //protocol: &mut [u8],
    //version: &mut [u8],
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
        let (mut request_head, mut request_body) = match request_head_body_split(buf) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };

        let (mut request_line, mut request_headers) = match request_line_header_split(buf) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };

        let mut request_line_items: [&mut [u8]; 3] = match request_line
            .split_mut(|c| *c == b' ')
            .collect::<Vec<&mut [u8]>>()
            .try_into() {
                Err(e) => return Err("Invalid request line".to_string()),
                Ok(items) => items,
        };

        request_line_items[0].make_ascii_uppercase();
        let method = match str::from_utf8(request_line_items[0]) {
            Err(e) => return Err(format!("Invalid method string: {}", e)),
            Ok(m) => m.to_string(),
        };

        let (mut document, mut query) = match split_once(request_line_items[1], b'?') {
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

        Ok(Self {
            method: method,
            document: document,
            query: query,
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
fn split_once(to_split: &mut [u8], by: u8) -> Result<(&mut [u8], Option<&mut [u8]>), String> {
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

    let (item, mut rest) = to_split.split_at_mut(found_idx);
    rest = rest.split_at_mut(1).1;
    Ok((item, Some(rest)))
}

/**
 * Find the index of the first crlf and return a tuple of two mutable string slices, the first
 * being the buffer slice up to the crlf, and the second being the slice content after the
 * clrf
 */
fn request_line_header_split(to_split: &mut [u8]) -> Result<(&mut [u8], Option<&mut [u8]>), String> {
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
        return Ok((to_split, None));
    }

    let (req_line, mut req_headers) = to_split.split_at_mut(crlf_start_idx);
    req_headers = req_headers.split_at_mut(2).1;
    Ok((req_line, Some(req_headers)))
}

/**
 * Find the index of the first double crlf and return a tuple of two mutable string slices, the
 * first being the slice content up to the double crlf, and the second being being the slice 
 * content after the double clrf
 */
fn request_head_body_split(to_split: &mut [u8]) -> Result<(&mut [u8], Option<&mut [u8]>), String>  {
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

    let (head, mut body) = to_split.split_at_mut(crlf_start_idx);
    body = body.split_at_mut(4).1;
    Ok((head, Some(body)))
}

