
use super::util::is_param_name_valid;

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

impl ToString for Cookie {
    fn to_string(&self) -> String {
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
        out
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
    
    /// Exposes the builder pattern
    pub fn builder() -> CookieBuilder {
        CookieBuilder::new()
    }
}

#[derive(Default)]
pub struct CookieBuilder {
    cookie: Cookie,
}

#[allow(dead_code)]
impl CookieBuilder {
    pub fn new() -> Self {
        Self {
            cookie: Cookie::new(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.cookie.name = name.to_owned();
        self
    }

    pub fn value(mut self, value: &str) -> Self {
        self.cookie.value = value.to_owned();
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

    pub fn domain(mut self, domain: &str) -> Self {
        self.cookie.domain = domain.to_owned();
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        self.cookie.path = path.to_owned();
        self
    }

    pub fn max_age(mut self, max_age: i64) -> Self {
        self.cookie.max_age = max_age;
        self
    }

    pub fn build(self) -> Cookie {
        self.cookie
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParseState {
    Name,
    Value,
}

/// Performs the state transition for the cookie parser
fn parse_cookies_state_transition(state: &mut ParseState, cookies: &mut Vec<Cookie>, cookie: &mut Cookie, builder: &mut String, component: &str) {
    for c in component.trim().chars() {
        match c {
            '=' => {
                if *state == ParseState::Name {
                    *builder = builder.trim().to_string();
                    if !builder.is_empty() && is_param_name_valid(builder) {
                        // end the current cookie and start a new one
                        if !cookie.name.is_empty() && !cookie.value.is_empty() {
                            cookies.push(cookie.clone());
                            *cookie = Cookie::new();
                        }

                        cookie.name = builder.clone();
                        *state = ParseState::Value;
                        builder.clear();
                    }
                } else {
                    builder.push(c);
                }
            }
            _ => builder.push(c),
        }
    }
}

/// Performs the state action for the cookie parser
fn parse_cookies_state_action(state: &mut ParseState, cookie: &mut Cookie, builder: &mut String) {
    *builder = builder.trim().to_string();
    match state {
        // should be name if no '=' was encountered
        ParseState::Name => { },
        ParseState::Value => {
            cookie.value = builder.replace(['"', '\\', ',', '\t', '\r', '\n', '\0'], "");
        },
    };

    *state = ParseState::Name;
    builder.clear();
}

/// Take a string containing arbitrary HTTP cookies and parse them into Cookie structs
/// note: cookies parsed this way will only have their name and value members filled out, as 
/// browsers do not echo the other components of the cookie in requests.
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

