
use crate::immortal::util::is_param_name_valid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SameSite {
    Undefined,
    No,
    Lax,
    Strict,
}

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub domain: String,
    pub path: String,
    pub max_age: i64,
}

impl Default for Cookie {
    fn default() -> Self {
        Self::new()
    }
}

impl Cookie {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            value: String::new(),
            secure: false,
            http_only: false,
            same_site: SameSite::Undefined,
            domain: String::new(),
            path: String::new(),
            max_age: -1i64,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParseState {
    Name,
    Value,
    SameSite,
    Domain,
    Path,
    Expires,
    MaxAge,
}

/**
 * Performs the state transition for the cookie parser
 */
fn parse_cookies_state_transition(state: &mut ParseState, cookies: &mut Vec<Cookie>, cookie: &mut Cookie, builder: &mut String, component: &str) {
    for c in component.trim().chars() {
        match c {
            '=' => {
                if *state == ParseState::Name {
                    *builder = builder.trim().to_string();
                    if !builder.is_empty() && is_param_name_valid(builder) {
                        if builder == "SameSite" {
                            *state = ParseState::SameSite;
                        } else if builder == "Domain" {
                            *state = ParseState::Domain;
                        } else if builder == "Path" {
                            *state = ParseState::Path;
                        } else if builder == "Expires" {
                            *state = ParseState::Expires;
                        } else if builder == "Max-Age" {
                            *state = ParseState::MaxAge;
                        } else {
                            // end the current cookie and start a new one
                            if !cookie.name.is_empty() && !cookie.value.is_empty() {
                                cookies.push(cookie.clone());
                                *cookie = Cookie::new();
                            }

                            cookie.name = builder.clone();
                            *state = ParseState::Value;
                        }
                        builder.clear();
                    }
                } else if *state == ParseState::Value {
                    builder.push(c);
                }
            }
            _ => builder.push(c),
        }
    }
}

/**
 * Performs the state action for the cookie parser
 */
fn parse_cookies_state_action(state: &mut ParseState, cookie: &mut Cookie, builder: &mut String) {
    *builder = builder.trim().to_string();
    match state {
        // should be name if no '=' was encountered
        ParseState::Name => {
            if builder == "Secure" {
                cookie.secure = true;
            } else if builder == "HttpOnly" {
                cookie.http_only = true;
            }
        },
        ParseState::Value => {
            cookie.value = builder.replace(['"', '\\', ',', '\t', '\r', '\n', '\0'], "");
        },
        ParseState::SameSite => {
            if builder == "Strict" {
                cookie.same_site = SameSite::Strict;
            } else if builder == "Lax" {
                cookie.same_site = SameSite::Lax;
            } else if builder == "None" {
                cookie.same_site = SameSite::No;
            } else {
                cookie.same_site = SameSite::Undefined;
            }
        },
        ParseState::Domain => {
            cookie.domain = builder.clone();
        },
        ParseState::Path => {
            cookie.path = builder.clone();
        },
        ParseState::Expires => {
            // more trouble than its worth parsing, use Max-Age instead
        },
        ParseState::MaxAge => {
            cookie.max_age = builder.parse::<i64>().unwrap_or(-1i64);
        },
    };

    *state = ParseState::Name;
    builder.clear();
}

/**
 * Take a string containing arbitrary HTTP cookies and parse them into Cookie structs
 */
pub fn parse_cookies(raw_cookies: &str) -> Vec<Cookie> {
    if raw_cookies.is_empty() { return Vec::new() }

    let mut state = ParseState::Name;
    let mut cookies: Vec<Cookie> = Vec::new();
    let mut cookie = Cookie::new();
    let mut builder = String::new();

    for component in raw_cookies.split(';') {
        parse_cookies_state_transition(&mut state, &mut cookies, &mut cookie, &mut builder, component);
        parse_cookies_state_action(&mut state, &mut cookie, &mut builder);
    }
    // collect last parsed item
    parse_cookies_state_action(&mut state, &mut cookie, &mut builder);

    // push cookie into the vector if its valid
    if !cookie.name.is_empty() && !cookie.value.is_empty() {
        cookies.push(cookie.clone());
    }

    cookies
}

