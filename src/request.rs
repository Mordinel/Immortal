
use std::fmt::Display;
use std::str::{self, Utf8Error};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::error;

use crate::cookie::{Cookie, parse_cookies};
use crate::util::*;

use debug_print::debug_eprintln;


/// Request contains the request representation that is serialised from the main HTTP request from
/// the socket.
#[derive(Debug, Default)]
pub struct Request<'buf> {
    pub body: Option<&'buf [u8]>,
    pub method: String,
    pub document: String,
    pub query: String,
    pub protocol: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub get: HashMap<String, String>,
    pub post: HashMap<String, String>,
    pub cookies: HashMap<String,Cookie>,
    
    pub host: String,
    pub user_agent: String,
    pub content_type: String,
    pub content_length: Option<usize>,
    pub peer_addr: Option<SocketAddr>,
}

#[derive(Debug)]
pub enum RequestError<'buf> {
    RequestLineMalformed(Vec<&'buf [u8]>),

    DocumentNotUtf8(Utf8Error),
    DocumentMalformed(&'buf [u8]),

    MethodNotUtf8(Utf8Error),

    QueryNotUtf8(Utf8Error),

    ProtoNotUtf8(Utf8Error),
    ProtoMalformed(&'buf [u8]),
    ProtoInvalid(&'buf [u8]),

    ProtoVersionNotUtf8(Utf8Error),
    ProtoVersionInvalid(&'buf [u8]),

    HeadersNotUtf8(Utf8Error),

    ContentLengthDiscrepancy {expected: usize, got: usize },

    PostParamsMalformed(&'buf [u8]),
}
impl Display for RequestError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for RequestError<'_> {}

#[allow(dead_code)]
impl<'buf> Request<'buf> {
    /// Construct a new request object using only a slice of u8
    pub fn from_slice(buf: &'buf [u8]) -> Result<Self, RequestError<'buf>> {
        Self::new(buf, None)
    }

    /// Create a default request object for a fail state
    pub fn bad() -> Self {
        Self::default()
    }

    /// Construct a new request object, parsing the request buffer
    pub fn new(
        buf: &'buf [u8],
        peer_addr: Option<&SocketAddr>
    ) -> Result<Self, RequestError<'buf>> {
        let (mut request_head, request_body) = request_head_body_split(buf);

        // ignore preceding clrf if they exist
        loop {
            request_head = match request_head.strip_prefix(b"\r\n") {
                Some(head) => head,
                None => break,
            };
        }

        let body = request_body;

        let (request_line, request_headers) = request_line_header_split(request_head);

        let request_line_items: [&[u8]; 3] = request_line
            .split(|c| *c == b' ')
            .collect::<Vec<&[u8]>>()
            .try_into()
            .map_err(RequestError::RequestLineMalformed)?;

        let method = str::from_utf8(&request_line_items[0].to_ascii_uppercase())
            .map_err(RequestError::MethodNotUtf8)?
            .to_string();

        let (document_slice, query) = split_once(request_line_items[1], b'?');

        let document = str::from_utf8(document_slice)
            .map_err(RequestError::DocumentNotUtf8)?
            .to_string();

        if !document.starts_with('/') {
            debug_eprintln!("ERROR: {document} does not start with /");
            return Err(RequestError::DocumentMalformed(document_slice));
        }

        let query = match query {
            None => "".to_string(),
            Some(thing) => str::from_utf8(thing)
                .map_err(RequestError::QueryNotUtf8)?
                .to_string(),
        };
        
        let proto_version_items: [&[u8]; 2] = match request_line_items[2]
            .split(|c| *c == b'/')
            .collect::<Vec<&[u8]>>()
            .try_into() {
                Err(_) => {
                    debug_eprintln!("ERROR: Invalid protocol string: {}", 
                        str::from_utf8(request_line_items[2])
                            .unwrap_or(&format!("{:?}", request_line_items[2])));
                    return Err(RequestError::ProtoMalformed(request_line_items[2]));
                },
                Ok(items) => items,
        };

        let protocol = str::from_utf8(proto_version_items[0])
            .map_err(RequestError::ProtoNotUtf8)?
            .to_string();

        if protocol != "HTTP" {
            debug_eprintln!("ERROR: Invalid protocol {protocol}");
            return Err(RequestError::ProtoInvalid(request_line));
        }

        let version = str::from_utf8(proto_version_items[1])
            .map_err(RequestError::ProtoVersionNotUtf8)?
            .trim_end_matches(|c| ['\r', '\n', '\0'].contains(&c))
            .to_string();

        if version != "1.1" {
            debug_eprintln!("ERROR: Invalid version {version}");
            return Err(RequestError::ProtoVersionInvalid(request_line));
        }

        let headers = parse_headers(request_headers.unwrap_or_default())
            .map_err(RequestError::HeadersNotUtf8)?;

        let host = collect_header(&headers, "Host");
        let user_agent = collect_header(&headers, "User-Agent");
        let content_type = collect_header(&headers, "Content-Type");
        let content_length = collect_header(&headers, "Content-Length");
        let cookies_raw = collect_header(&headers, "Cookie");

        let content_length = match content_length.parse::<usize>() {
            Err(_) => None,
            Ok(len) => {
                if let Some(mut body) = body {
                    body = match body.get(..len) {
                        Some(slice) => slice,
                        None => {
                            debug_eprintln!("ERROR: Content-Length discrepancy {} != {}",
                                len, body.len());
                            return Err(RequestError::ContentLengthDiscrepancy { expected: len, got: body.len() });
                        },
                    };
                    if len != body.len() {
                        debug_eprintln!("ERROR: Content-Length discrepancy {} != {}", len, body.len());
                        return Err(RequestError::ContentLengthDiscrepancy { expected: len, got: body.len() });

                    }
                    Some(len)
                } else {
                    None
                }
            },
        };

        let get = match parse_parameters(&query) {
            Ok(g) => g,
            Err(_) => {
                debug_eprintln!("ERROR: Invalid get parameters: {}", 
                    format!("{:?}", query));
                HashMap::new()
            }
        };

        let post = if method == "POST"
                && content_type == "application/x-www-form-urlencoded"
                && content_length.is_some() && body.is_some() {
            let body = body.unwrap();
            match parse_parameters(str::from_utf8(body).unwrap_or_default()) {
                Ok(p) => p,
                Err(_) => {
                    debug_eprintln!("ERROR: Invalid post parameters: {}", 
                        str::from_utf8(body).unwrap_or(&format!("{:?}", body)));
                    return Err(RequestError::PostParamsMalformed(body));
                }
            }
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
            content_type,
            content_length,
            peer_addr: peer_addr.copied(),
        })
    }
    
    /// looks up HTTP headers in the internal hashmap and returns its value
    pub fn header(&self, key: &str) -> Option<&str> {
        match self.headers.get(&key.to_ascii_uppercase()) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }

    /// looks up cookies keys and returns its value
    pub fn cookie(&self, key: &str) -> Option<&Cookie> {
        self.cookies.get(key)
    }

    /// looks up get parameters and returns its value
    pub fn get(&self, key: &str) -> Option<&str> {
        match self.get.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }

    /// looks up post parameters and returns its value
    pub fn post(&self, key: &str) -> Option<&str> {
        match self.post.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }
}

/// Returns a hashmap of http cookies with the name value as the key
fn get_cookies(cookies_raw: &str) -> HashMap<String, Cookie> {
    let mut cookies = HashMap::new();

    let cookie_vec = parse_cookies(cookies_raw);
    for cookie in cookie_vec {
        cookies.insert(String::from(&cookie.name), cookie);
    }

    cookies
}

/// Accepts a hashmap and a key, returns the key value or an empty string
/// [[[ASSUMES `key` IS A VALID NON EMPTY HEADER KEY]]]: handling errors from this will be annoying.
fn collect_header(headers: &HashMap<String, String>, key: &str) -> String {
    let key = key.to_ascii_uppercase();
    match headers.get(&key) {
        None => String::new(),
        Some(thing) => thing.to_string(),
    }
}

/// Find the index of the first crlf and return a tuple of two mutable string slices, the first
/// being the buffer slice up to the crlf, and the second being the slice content after the
/// clrf
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
        let line_cleaned = match to_split.strip_suffix(b"\r\n") {
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

/// Find the index of the first double crlf and return a tuple of two mutable string slices, the
/// first being the slice content up to the double crlf, and the second being being the slice 
/// content after the double clrf
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

