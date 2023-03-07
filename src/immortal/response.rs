
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::immortal::Request;

#[derive(Debug)]
pub struct Response<'a> {
    pub body: Vec<u8>,
    pub code: &'a str,
    pub status: &'a str,
    pub protocol: &'a str,
    pub method: String,
    pub headers: HashMap<&'a str, String>,
    // pub cookies: Vec<Cookie>,
}

impl Response<'_> {
    /**
     *  Constructs a default response based on the passed request.
     */
    pub fn new(req: &Request) -> Self {
        let mut headers: HashMap<&str, String> = HashMap::new();
        let now: DateTime<Utc> = Utc::now();

        // default headers
        headers.insert("Date", now.format("%a, %d %b %Y %H:%M:%S").to_string());
        headers.insert("Connection", match req.keep_alive {
            true => "keep-alive".to_string(),
            false => "close".to_string(),
        });
        headers.insert("Content-Type", "text/html".to_string());

        Self {
            body: vec![],
            code: "200",
            status: "OK",
            protocol: "HTTP/1.1",
            method: req.method.clone(),
            headers,
        }
    }

    /**
     * Constructs a default error response
     */
    pub fn bad() -> Self {
        let mut headers: HashMap<&str, String> = HashMap::new();
        let now: DateTime<Utc> = Utc::now();

        // default headers
        headers.insert("Date", now.format("%a, %d %b %Y %H:%M:%S").to_string());
        headers.insert("Connection",  "close".to_string());
        headers.insert("Content-Type", "text/html".to_string());

        Self {
            body: vec![],
            code: "400",
            status: "BAD REQUEST",
            protocol: "HTTP/1.1",
            method: "GET".to_string(),
            headers,
        }
    }

    /**
     * Generates the serial data for an HTTP response using the object internal state
     */
    pub fn serialize(&mut self) -> Vec<u8> {
        let mut serialized = vec![];
        let statuses: HashMap<String, String> = HashMap::from([ // TODO: Make this static
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
        ]);

        let mut status = match statuses.get(self.code) {
            None => self.status,
            Some(thing) => thing,
        };

        if status.is_empty() {
            self.code = "500";
            status = match statuses.get(self.code) {
                None => "INTERNAL SERVER ERROR",
                Some(thing) => thing,
            };
            self.headers.insert("Content-Type", "text/html".to_string());
            self.body = format!("<h1>500: {}</h1>", status).into_bytes();
        }

        // emit the status line
        serialized.append(&mut format!("{} {} {}\r\n", &self.protocol, &self.code, &status).into_bytes());

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

    pub fn header(&self, key: &str) -> Option<&str> {
        match self.headers.get(key) {
            None => None,
            Some(thing) => Some(thing.as_str()),
        }
    }
}
