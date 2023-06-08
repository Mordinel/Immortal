use std::collections::HashMap;
use openssl::rand::rand_bytes;

use super::request::Cookies;

#[allow(dead_code)]
pub struct Session {
    id: String,
    data: HashMap<String, String>,
}

impl Session {
    fn new(id: &str) -> Session {
        Session {
            id: id.to_owned(),
            data: HashMap::new(),
        }
    }
}

pub type SessionStore = HashMap<String, Session>;

/// provides APIs to interact with user session stores.
#[derive(Default)]
pub struct SessionManager {
    store: SessionStore,
}

fn to_hex_string(buf: [u8;28]) -> String {
    let mut out = String::new();
    for b in buf {
        out.push_str(format!("{b:02x}").as_str());
    }
    out
}

impl SessionManager {
    pub fn new() -> SessionManager {
        SessionManager {
            store: HashMap::new(),
        }
    }

    /// creates a persistent session between requests
    /// returns the session ID as a string
    pub fn create_session(&mut self) -> Option<String> {
        // generate a random ID
        let mut buf = [0u8;28];
        rand_bytes(&mut buf).unwrap();
        let out = to_hex_string(buf);

        if self.store.contains_key(&out) { return None }
        self.store.insert(out.to_owned(), Session::new(&out));
        Some(out)
    }

    /// writes `value` to the key-value session store as `key` for the `session_id` session store.
    pub fn write_session(&mut self, session_id: &str, key: &str, value: &str) -> bool {
        if let Some(session) = self.store.get_mut(session_id) {
            if value.is_empty() {
                session.data.remove(key);
                session.data.shrink_to_fit();
            } else {
                session.data.insert(key.to_owned(), value.to_owned());
            }
            return true;
        }
        false
    }

    /// reads the value associated with `key` for the `session_id` session store.
    pub fn read_session(&self, session_id: &str, key: &str) -> Option<String> {
        if let Some(session) = self.store.get(session_id) {
            if let Some(value) = session.data.get(key) {
                return Some(value.to_owned());
            }
        }
        None
    }

    /// empties the session store for `session_id`
    pub fn clear_session(&mut self, session_id: &str) {
        if let Some(session) = self.store.get_mut(session_id) {
            session.data.clear();
            session.data.shrink_to_fit();
        }
    }

    /// removes the session store for `session_id`
    pub fn delete_session(&mut self, session_id: &str) {
        self.store.remove(session_id);
        self.store.shrink_to_fit();
    }

    /// checks if a session store for `session_id` exists
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.store.contains_key(session_id)
    }

    /// tries to get an existing session 
    /// if a session does not exist, a session is created.
    /// The returned tuple contains the session id and a boolean, the boolean is true if the
    /// created session is new, if it is false, the session id is for an existing session.
    pub fn get_or_create_session(&mut self, cookies: &Cookies) -> Option<(String, bool)>{
        let mut session_id = String::new();

        let mut is_new_session = false;
        if cookies.contains_key("id") {
            session_id = match cookies.get("id") {
                None => String::new(),
                Some(thing) => {
                    thing.value.clone()
                }
            };
            if !self.session_exists(&session_id) {
                session_id = match self.create_session() {
                    None => return None,
                    Some(thing) => thing,
                };
                is_new_session = true;
            }
        } else if !self.session_exists(&session_id) {
            session_id = match self.create_session() {
                None => return None,
                Some(thing) => thing,
            };
            is_new_session = true;
        }
        Some((session_id, is_new_session))
    }
}

