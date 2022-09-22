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
use std::io;
use std::io::ErrorKind;
use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::immortal::util::*;
use crate::immortal::cookie::{Cookie, parse_cookies};

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
    pub post: HashMap<String, String>,
    pub cookies: HashMap<String, Cookie>,
    
    pub host: String,
    pub user_agent: String,
    pub connection: String,
    pub content_type: String,
    pub content_length: usize,
    pub keep_alive: bool,
}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        if self.body != other.body { return false }
        if self.method != other.method { return false }
        if self.document != other.document { return false }
        if self.query != other.query { return false }
        if self.protocol != other.protocol { return false }
        if self.version != other.version { return false }

        if self.host != other.host { return false }
        if self.user_agent != other.user_agent { return false }
        if self.connection != other.connection { return false }
        if self.content_type != other.content_type { return false }
        if self.content_length != other.content_length { return false }
        if self.keep_alive != other.keep_alive { return false }

        if self.get.len() != other.get.len() { return false }
        if self.post.len() != other.post.len() { return false }
        if self.headers.len() != other.headers.len() { return false }

        for (key, value) in &self.get {
            match other.get.get(key) {
                None => return false,
                Some(thing) => {
                    if value != thing { return false }
                },
            }
        }

        for (key, value) in &self.post {
            match other.post.get(key) {
                None => return false,
                Some(thing) => {
                    if value != thing { return false }
                },
            }
        }

        for (key, value) in &self.headers {
            match other.headers.get(key) {
                None => return false,
                Some(thing) => {
                    if value != thing { return false }
                },
            }
        }

        true
    }
}

impl Request {
    /**
     * Construct a new request object, parsing the request buffer
     */
    pub fn new(buf: &mut [u8]) -> Result<Self> {
        // parse body
        let (request_head, request_body) = request_head_body_split(buf);

        let body = match request_body {
            None => vec![],
            Some(thing) => thing.to_vec(),
        };

        // split request line and header chunk
        let (request_line, request_headers) = request_line_header_split(request_head);

        // split to each major component of a request line
        let request_line_items: [&[u8]; 3] = match request_line
            .split(|c| *c == b' ')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err(anyhow!(io::Error::new(ErrorKind::InvalidInput, "Invalid request line"))),
                Ok(items) => items,
        };

        // parse method
        let method = str::from_utf8(&request_line_items[0].to_ascii_uppercase())?.to_string();

        // parse document and query
        let (document, query) = split_once(request_line_items[1], b'?');

        let document = str::from_utf8(document)?.to_string();

        let query = match query {
            None => "".to_string(),
            Some(thing) => str::from_utf8(thing)?.to_string(),
        };
        
        // parse protocol/version components
        let proto_version_items: [&[u8]; 2] = match request_line_items[2]
            .split(|c| *c == b'/')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => return Err(anyhow!(io::Error::new(ErrorKind::InvalidInput, "Invalid proto string"))),
                Ok(items) => items,
        };

        // parse protocol string
        let protocol = str::from_utf8(proto_version_items[0])?.to_string();

        if protocol != "HTTP" {
            return Err(anyhow!(io::Error::new(ErrorKind::InvalidInput, "Invalid protocol in proto string")));
        }

        // parse protocol version string
        let version = str::from_utf8(proto_version_items[1])?
            .trim_end_matches(|c| c == '\r' || c == '\n' || c == '\0')
            .to_string();

        if version != "1.1" {
            return Err(anyhow!(io::Error::new(ErrorKind::InvalidInput, "Invalid version in proto string")));
        }

        // parse headers
        let headers = parse_headers(request_headers.unwrap_or_default())?;

        // collect common headers from the `headers` HashMap
        let host = collect_header(&headers, "Host");
        let user_agent = collect_header(&headers, "User-Agent");
        let connection = collect_header(&headers, "Connection");
        let content_type = collect_header(&headers, "Content-Type");
        let content_length = collect_header(&headers, "Content-Length");
        let cookies_raw = collect_header(&headers, "Cookie");

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

        let get = parse_parameters(&query);

        let post = if method == "POST" && content_type == "application/x-www-form-urlencoded" {
            parse_parameters(&str::from_utf8(&body).unwrap_or_default())
        } else {
            HashMap::new()
        };

        let cookies = get_cookies(&cookies_raw);

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
            post,
            cookies,
            host,
            user_agent,
            connection,
            content_type,
            content_length,
            keep_alive,
        })
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        match self.headers.get(&key.to_ascii_uppercase()) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }

    pub fn cookie(&self, key: &str) -> Option<&Cookie> {
        self.cookies.get(key)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match self.get.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }

    pub fn post(&self, key: &str) -> Option<&str> {
        match self.post.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }
}

/**
 * Returns a hashmap of http cookies with the name value as the key
 */
fn get_cookies(cookies_raw: &str) -> HashMap<String, Cookie> {
    let mut cookies = HashMap::new();

    let cookie_vec = parse_cookies(cookies_raw);
    for cookie in cookie_vec {
        cookies.insert(String::from(&cookie.name), cookie);
    }

    cookies
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
 * Find the index of the first crlf and return a tuple of two mutable string slices, the first
 * being the buffer slice up to the crlf, and the second being the slice content after the
 * clrf
 */
fn request_line_header_split(to_split: &[u8]) -> (&[u8], Option<&[u8]>) {
    let mut found_cr = false;
    let mut found_lf = false;
    let mut crlf_start_idx = 0;

    // iterate over the slice and get the index of the first crlf
    for (idx, byte) in to_split.iter().enumerate() {
        if *byte == b'\r' {
            crlf_start_idx = idx;
            found_cr = true;
            continue;
        }
        if found_cr && *byte == b'\n' {
            found_lf = true;
            break;
        }
        crlf_start_idx = 0;
        found_cr = false;
    }

    // if no crlf was found or its at index 0, strip off crlf if possible and then return it
    if crlf_start_idx == 0 || !found_cr || !found_lf {
        let line_cleaned = match to_split.strip_suffix(&[b'\r', b'\n']) {
            None => return (to_split, None),
            Some(thing) => thing,
        };
        return (line_cleaned, None);
    }

    // build the returned tuple excluding the crlf in the data
    let (req_line, req_headers) = to_split.split_at(crlf_start_idx);
    let req_headers = req_headers.split_at(2).1;
    (req_line, Some(req_headers))
}

/**
 * Find the index of the first double crlf and return a tuple of two mutable string slices, the
 * first being the slice content up to the double crlf, and the second being being the slice 
 * content after the double clrf
 */
fn request_head_body_split(to_split: &[u8]) -> (&[u8], Option<&[u8]>)  {
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
        return (to_split, None);
    }

    // if exited without fulfilling 2 crlf's, return it
    if crlf_count != 2 {
        return (to_split, None);
    }

    // build the returned tuple excluding the double crlf in the data
    let (head, body) = to_split.split_at(crlf_start_idx);
    let body = body.split_at(4).1;
    (head, Some(body))
}

