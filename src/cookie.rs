
use std::fmt::Display;

use crate::util::KVParser;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SameSite {
    Undefined,
    No,
    Lax,
    Strict,
}

/// Cookies are a high-level representation used for serialisation and deserialisation of browser
/// cookies.
#[derive(Debug, Clone)]
pub struct Cookie<'buf> {
    pub name: &'buf str,
    pub value: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub domain: &'buf str,
    pub path: &'buf str,
    pub max_age: i64,
}

impl<'buf> Default for Cookie<'buf> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'buf> Display for Cookie<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = format!("{}={}", self.name, self.value);
        if self.secure {
            out += "; Secure";
        }
        if self.http_only {
            out += "; HttpOnly";
        }
        out += match self.same_site {
            SameSite::Undefined => "",
            SameSite::No => "; SameSite=No",
            SameSite::Lax => "; SameSite=Lax",
            SameSite::Strict => "; SameSite=Strict",
        };
        if !self.domain.is_empty() {
            out += &format!("; Domain={}", self.domain);
        }
        if !self.path.is_empty() {
            out += &format!("; Path={}", self.path);
        }
        if self.max_age > 0 {
            out += &format!("; Max-Age={}", self.max_age);
        }
        write!(f, "{out}")
    }
}

impl<'buf> Cookie<'buf> {
    pub fn new() -> Self {
        Self {
            name: "",
            value: "".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Undefined,
            domain: "",
            path: "",
            max_age: -1i64,
        }
    }
    
    /// Exposes the builder pattern
    pub fn builder() -> CookieBuilder<'buf> {
        CookieBuilder::new()
    }
}

#[derive(Default)]
pub struct CookieBuilder<'buf> {
    cookie: Cookie<'buf>,
}

#[allow(dead_code)]
impl<'buf> CookieBuilder<'buf> {
    pub fn new() -> Self {
        Self {
            cookie: Cookie::new(),
        }
    }

    pub fn name(mut self, name: &'buf str) -> Self {
        self.cookie.name = name;
        self
    }

    pub fn value(mut self, value: &str) -> Self {
        self.cookie.value = value.to_string();
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.cookie.secure = secure;
        self
    }

    pub fn http_only(mut self, http_only: bool) -> Self {
        self.cookie.http_only = http_only;
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.cookie.same_site = same_site;
        self
    }

    pub fn domain(mut self, domain: &'buf str) -> Self {
        self.cookie.domain = domain;
        self
    }

    pub fn path(mut self, path: &'buf str) -> Self {
        self.cookie.path = path;
        self
    }

    pub fn max_age(mut self, max_age: i64) -> Self {
        self.cookie.max_age = max_age;
        self
    }

    pub fn build(self) -> Cookie<'buf> {
        self.cookie
    }
}

/// Take a string containing arbitrary HTTP cookies and parse them into Cookie structs
///
/// note: cookies parsed this way will only have their name and value members filled out, as 
/// browsers do not echo the other components of the cookie in requests.
pub fn parse_cookies(raw_cookies: &str) -> Vec<Cookie> {
    if raw_cookies.is_empty() { return Vec::new() }

    let mut cookies: Vec<Cookie> = Vec::new();
    let mut cookie = Cookie::new();

    for component in raw_cookies.split(';') {
        let mut pp = KVParser::new(component.trim());
        if let Some((key, value)) = pp.cookie_kv_pair() {
            cookie.name = key;
            cookie.value = value.to_string();
            cookies.push(cookie);
            cookie = Cookie::new();
        }
    }

    cookies
}

