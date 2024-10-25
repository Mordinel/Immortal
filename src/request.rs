
use std::fmt::Display;
use std::str::{self, Utf8Error};
use std::net::SocketAddr;
use std::error;

use crate::cookie::{Cookie, parse_cookies};
use crate::util::*;

use debug_print::{debug_eprintln, debug_println};


/// Request contains the request representation that is serialised from the main HTTP request from
/// the socket.
#[derive(Debug, Default)]
pub struct Request<'buf> {
    pub body: Option<&'buf [u8]>,
    pub method: &'buf str,
    pub document: &'buf str,
    pub query_raw: &'buf str,
    pub protocol: &'buf str,
    pub version: &'buf str,
    pub header_raw_lines: Vec<&'buf str>,

    headers: Vec<(&'buf str, &'buf str)>,
    get: Vec<(&'buf str, &'buf str)>,
    post: Vec<(&'buf str, &'buf str)>,
    cookies: Vec<Cookie<'buf>>,

    host: Option<&'buf str>,
    user_agent: Option<&'buf str>,
    content_type: Option<&'buf str>,
    content_length: Option<usize>,

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

        let method = str::from_utf8(request_line_items[0])
            .map_err(RequestError::MethodNotUtf8)?;

        let (document_slice, query) = split_once(request_line_items[1], b'?');

        let document = str::from_utf8(document_slice)
            .map_err(RequestError::DocumentNotUtf8)?;

        if !document.starts_with('/') {
            debug_eprintln!("ERROR: {document} does not start with /");
            return Err(RequestError::DocumentMalformed(document_slice));
        }

        let query = match query {
            None => "",
            Some(thing) => str::from_utf8(thing)
                .map_err(RequestError::QueryNotUtf8)?
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
            .map_err(RequestError::ProtoNotUtf8)?;

        if protocol != "HTTP" {
            debug_eprintln!("ERROR: Invalid protocol {protocol}");
            return Err(RequestError::ProtoInvalid(request_line));
        }

        let version = str::from_utf8(proto_version_items[1])
            .map_err(RequestError::ProtoVersionNotUtf8)?
            .trim_end_matches(|c| ['\r', '\n', '\0'].contains(&c));

        if version != "1.1" {
            debug_eprintln!("ERROR: Invalid version {version}");
            return Err(RequestError::ProtoVersionInvalid(request_line));
        }

        let header_raw_lines = str::from_utf8(request_headers.unwrap_or_default())
            .map_err(RequestError::HeadersNotUtf8)?
            .split(&"\r\n")
            .collect::<Vec<_>>();

        let headers_len = header_raw_lines.len();

        // emit a complete Request object
        Ok(Self {
            body,
            method,
            document,
            query_raw: query,
            protocol,
            version,
            header_raw_lines,
            headers: Vec::with_capacity(headers_len),
            get: Vec::new(),
            post: Vec::new(),
            cookies: Vec::new(),
            host: None,
            user_agent: None,
            content_type: None,
            content_length: None,
            peer_addr: peer_addr.copied(),
        })
    }
    
    pub fn host(&mut self) -> Option<&'buf str> {
        if let Some(host) = self.host {
            Some(host)
        } else if let Some(host) = self.header("Host") {
            self.host = Some(host);
            Some(host)
        } else {
            None
        }
    }

    pub fn user_agent(&mut self) -> Option<&'buf str> {
        if let Some(ua) = self.user_agent {
            Some(ua)
        } else if let Some(ua) = self.header("User-Agent") {
            self.user_agent = Some(ua);
            Some(ua)
        } else {
            None
        }
    }

    pub fn content_type(&mut self) -> Option<&'buf str> {
        if let Some(ct) = self.content_type {
            Some(ct)
        } else if let Some(ct) = self.header("Content-Type") {
            self.content_type = Some(ct);
            Some(ct)
        } else {
            None
        }
    }

    pub fn content_length(&mut self) -> Option<usize> {
        if let Some(cl) = self.content_length {
            Some(cl)
        } else if let Some(cl) = self.header("Content-Length") {
            let cl = cl.parse::<usize>().ok();
            self.content_length = cl;
            cl
        } else {
            None
        }
    }

    /// looks up HTTP headers and returns
    /// headers are not parsed until they are needed
    pub fn header(&mut self, key: &str) -> Option<&'buf str> {
        if self.header_raw_lines.is_empty() {
            return None;
        }
        if let Some((_k, v)) = self.headers.iter()
                .find(|(k, _v)| *k == key) {
            return Some(v);
        } else if let Some(raw) = self.header_raw_lines.iter()
                .find(|line| line.find(": ").map(|idx| &line[..idx] == key).unwrap_or(false)) {
            if let Some((key, value)) = parse_header(raw) {
                self.headers.push((key, value));
                return Some(value);
            }
        }
        None
    }

    /// looks up cookies keys and returns its value
    /// cookies are not parsed until they are needed, will parse headers too.
    pub fn cookie(&mut self, key: &str) -> Option<&Cookie<'buf>> {
        if self.header_raw_lines.is_empty() {
            return None;
        }
        if self.cookies.is_empty() {
            if let Some(cookies_raw) = self.header("Cookie") {
                let cookies = parse_cookies(cookies_raw);
                if cookies.is_empty() {
                    return None;
                }
                self.cookies = cookies;
            } else {
                return None;
            }
        }
        self.cookies.iter()
            .find(|c| c.name == key)
    }

    /// looks up get parameters and returns its value
    /// will parse all parameters on the first call.
    pub fn get(&mut self, key: &str) -> Option<&str> {
        if self.query_raw.is_empty() {
            return None;
        }
        if self.get.is_empty() {
            if let Ok(get) = parse_parameters(self.query_raw) {
                if get.is_empty() {
                    return None;
                }
                self.get = get;
            } else {
                return None;
            }
        }
        self.get.iter()
            .find(|(k, _v)| *k == key)
            .map(|(_k, v)| *v)
    }

    /// looks up post parameters and returns its value
    /// will parse the content_type and content_len header on the first call.
    pub fn post(&mut self, key: &str) -> Option<&str> {
        // method must be POST
        if self.method != "POST" {
            return None;
        }
        // body must exist
        self.body?;

        // if post is empty, go about and parse the POST values from the request body.
        if self.post.is_empty() {
            // must have a content length
            if let Some(content_len) = self.content_length() {
                // and it must be nonzero
                if content_len == 0 {
                    return None;
                }
                // and there must be a content type
                if let Some(content_type) = self.content_type() {
                    // and the content type must be application/x-www-form-urlencoded
                    if content_type != "application/x-www-form-urlencoded" {
                        return None;
                    }
                    // and there must be a body
                    if let Some(body) = self.body {
                        // and the body, up to the content length, must be UTF-8
                        if let Ok(body) = str::from_utf8(body.get(0..content_len)?) {
                            // and the body is to be treated the same as GET query parameters
                            match parse_parameters(body) {
                                Ok(params) if params.is_empty() => {
                                    return None;
                                }
                                Ok(params) => {
                                    // and then, search the parsed POST values for the `key`
                                    self.post = params;
                                    // and return it if it exists.
                                    return self.post.iter()
                                        .find(|(k, _v)| *k == key)
                                        .map(|(_k, v)| *v);
                                    },
                                Err(_err) => {
                                    debug_println!("ERROR: Invalid post parameters: {body}: {_err}");
                                },
                            }
                        }
                    }
                }
            }
        } else {
            // otherwise, look up the requested POST value.
            return self.post.iter()
                .find(|(k, _v)| *k == key)
                .map(|(_k, v)| *v);
        }
        None
    }
}

/// Find the index of the first crlf and return a tuple of two mutable string slices, the first
/// being the buffer slice up to the crlf, and the second being the slice content after the clrf
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

