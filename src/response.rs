
use std::rc::Rc;
use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::Arc;

use crate::session::SessionManager;
use crate::request::Request;
use crate::cookie::Cookie;

use debug_print::debug_eprintln;
use lazy_static::lazy_static;
use chrono::{DateTime, Utc};
use uuid::Uuid;

lazy_static! {
    static ref STATUSES: HashMap<String, String> = HashMap::from([
            ( "200".to_string(), "OK".to_string() ),
            ( "301".to_string(), "MOVED PERMANENTLY".to_string() ),
            ( "302".to_string(), "FOUND".to_string() ),
            ( "308".to_string(), "PERMANENT REDIRECT".to_string() ),
            ( "400".to_string(), "BAD REQUEST".to_string() ),
            ( "401".to_string(), "UNAUTHORIZED".to_string() ),
            ( "403".to_string(), "FORBIDDEN".to_string() ),
            ( "404".to_string(), "NOT FOUND".to_string() ),
            ( "411".to_string(), "LENGTH REQUIRED".to_string() ),
            ( "413".to_string(), "PAYLOAD TOO LARGE".to_string() ),
            ( "414".to_string(), "URI TOO LONG".to_string() ),
            ( "418".to_string(), "I AM A TEAPOT".to_string() ),
            ( "426".to_string(), "UPGRADE REQUIRED".to_string() ),
            ( "451".to_string(), "UNAVAILABLE FOR LEGAL REASONS".to_string() ),
            ( "500".to_string(), "INTERNAL SERVER ERROR".to_string() ),
            ( "501".to_string(), "NOT IMPLEMENTED".to_string() ),
            ( "505".to_string(), "HTTP VERSION NOT SUPPORTED".to_string() ),
        ]);
}

#[derive(Debug)]
pub struct Response<'req> {
    pub body: Vec<u8>,
    pub code: &'req str,
    pub status: &'req str,
    pub protocol: &'req str,
    pub method: &'req str,
    pub headers: HashMap<&'req str, String>,
    pub cookies: Vec<Cookie<'req>>,
}

impl<'req> Response<'req> {
    /// Constructs a default response based on the passed request.
    pub fn new(
        req: Rc<RefCell<Request<'req>>>,
        session_manager: Arc<SessionManager>,
        session_id: &mut Uuid
    ) -> Self {
        let mut headers: HashMap<&str, String> = HashMap::new();

        // default headers
        headers.insert("Connection", "close".to_string());
        headers.insert("Content-Type", "text/html".to_string());

        let sm_is_enabled = session_manager.is_enabled();
        if sm_is_enabled {
            if let Some(cookie) = req.borrow_mut().cookie("id") {
                if let Ok(id) = cookie.value.parse::<Uuid>() {
                    *session_id = id;
                }
            }
        }

        let sm_should_gen_id = session_manager.is_enabled() 
                && !session_manager.add_session(*session_id) 
                && !session_manager.session_exists(*session_id);

        let mut should_add_cookie = false;
        if sm_should_gen_id {
            *session_id = SessionManager::generate_id();
            should_add_cookie = true;
        }

        let mut cookies: Vec<Cookie> = Vec::new();
        let sm_is_enabled = session_manager.is_enabled();
        if sm_is_enabled && should_add_cookie && !session_id.is_nil() {
            let cookie = Cookie::builder()
                .name("id")
                .value(session_id.to_string().as_str())
                .http_only(true)
                .build();
            cookies.push(cookie);
        }

        Self {
            body: vec![],
            code: "200",
            status: "OK",
            protocol: "HTTP/1.1",
            method: req.borrow_mut().method,
            headers,
            cookies,
        }
    }

    /// Constructs a default error response
    pub fn bad() -> Self {
        let mut headers: HashMap<&str, String> = HashMap::new();

        // default headers
        headers.insert("Connection",  "close".to_string());
        headers.insert("Content-Type", "text/html".to_string());

        Self {
            body: vec![],
            code: "400",
            status: "BAD REQUEST",
            protocol: "HTTP/1.1",
            method: "GET",
            headers,
            cookies: Vec::new(),
        }
    }

    /// Generates the serial data for an HTTP response using the object internal state
    pub fn serialize(&mut self) -> Vec<u8> {
        let mut serialized = vec![];

        let mut status = match STATUSES.get(self.code) {
            None => self.status,
            Some(thing) => thing,
        };

        if status.is_empty() {
            debug_eprintln!("ERROR: No default status string for HTTP {}, sending 500", self.code);
            self.code = "500";
            status = match STATUSES.get(self.code) {
                None => "INTERNAL SERVER ERROR",
                Some(thing) => thing,
            };
            self.headers.insert("Content-Type", "text/html".to_string());
            self.body = format!("<h1>500: {}</h1>", status).into_bytes();
        }

        if !self.cookies.is_empty() {
            self.headers.insert("Set-Cookie", self.cookies.iter()
                                .map(|c| c.to_string())
                                .intersperse("; ".to_string())
                                .reduce(|acc, c| acc + &c).unwrap());
        }

        // emit the status line
        serialized.append(&mut format!("{} {} {}\r\n", &self.protocol, &self.code, &status).into_bytes());

        let now: DateTime<Utc> = Utc::now();
        self.headers.insert("Date", now.format("%a, %d %b %Y %H:%M:%S").to_string());

        // emit headers
        for (key, value) in self.headers.iter() {
            if !key.is_empty() {
                serialized.append(&mut format!("{}: {}\r\n", &key, &value).into_bytes());
            }
        }

        // output content or not depending on the request method
        if self.method != "HEAD" {
            serialized.append(&mut format!("Content-Length: {}\r\n\r\n", &self.body.len()).into_bytes());
            serialized.append(&mut self.body);
        } else {
            serialized.append(&mut "Content-Length: 0\r\n\r\n".to_string().into_bytes());
        }

        serialized
    }

    /// looks up headers and returns it
    pub fn header(&self, key: &str) -> Option<&str> {
        match self.headers.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }

    pub fn is_redirect(&self) -> bool {
        let mut cases = 0;
        if self.code.starts_with('3') {
            cases += 1;
        }
        if self.header("Location").is_some() {
            cases += 1;
        }
        cases == 2
    }
}

