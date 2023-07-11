use std::{collections::HashMap, time::{Instant, Duration}};
use debug_print::debug_println;
use openssl::rand::rand_bytes;

use super::request::Cookies;

#[allow(dead_code)]
pub struct Session {
    id: String,
    data: HashMap<String, String>,
    last_interacted: Instant,
}

impl Session {
    fn new(id: &str) -> Session {
        Session {
            id: id.to_owned(),
            data: HashMap::new(),
            last_interacted: Instant::now(),
        }
    }
}

pub type SessionStore = HashMap<String, Session>;

/// provides APIs to interact with user session stores.
pub struct SessionManager {
    store: SessionStore,
    expiry_duration: Duration,
    prune_duration: Duration,
    last_prune: Instant,
}

fn to_hex_string(buf: [u8;28]) -> String {
    let mut out = String::new();
    for b in buf {
        out.push_str(format!("{b:02x}").as_str());
    }
    out
}

fn session_id_is_good(session_id: &str) -> bool {
    if session_id.len() != 56 {
        return false;
    }
    for chr in session_id.chars() {
        match chr {
            '0'..='9' | 'a'..='f' => {},
            _ => return false,
        }
    }
    true
}

impl Default for SessionManager {
    fn default() -> Self {
        SessionManager::new(Duration::from_secs(60*60), Duration::from_secs(60*10))
    }
}

impl SessionManager {
    pub fn new(expiry_duration: Duration, prune_duration: Duration) -> SessionManager {
        SessionManager {
            store: HashMap::new(),
            expiry_duration,
            prune_duration,
            last_prune: Instant::now(),
        }
    }

    /// sets the expiry duration for sessions
    pub fn set_expiry_duration(&mut self, duration: Duration) {
        self.expiry_duration = duration;
    }

    /// sets the try_prune duration for sessions
    pub fn set_prune_duration(&mut self, duration: Duration) {
        self.prune_duration = duration;
    }

    /// generates a new session id without storing a session
    pub fn generate_id() -> String {
        let mut buf = [0u8;28];
        rand_bytes(&mut buf).unwrap();
        to_hex_string(buf)
    }

    /// generates a new session and returns the ID
    pub fn create_session(&mut self) -> String {
        let mut out;
        loop {
            out = Self::generate_id();
            if !self.store.contains_key(&out) {
                break;
            }
        }
        self.store.insert(out.to_owned(), Session::new(&out));
        self.try_prune();
        out
    }

    /// writes `value` to the key-value session store as `key` for the `session_id` session store.
    pub fn write_session(&mut self, session_id: &str, key: &str, value: &str) -> bool {
        if let Some(session) = self.store.get_mut(session_id) {
            session.last_interacted = Instant::now();
            if value.is_empty() {
                session.data.remove(key);
                session.data.shrink_to_fit();
            } else {
                session.data.insert(key.to_owned(), value.to_owned());
            }
            return true;
        }
        self.try_prune();
        false
    }

    /// reads the value associated with `key` for the `session_id` session store.
    pub fn read_session(&mut self, session_id: &str, key: &str) -> Option<String> {
        if let Some(session) = self.store.get_mut(session_id) {
            session.last_interacted = Instant::now();
            if let Some(value) = session.data.get(key) {
                return Some(value.to_owned());
            }
        }
        self.try_prune();
        None
    }

    /// empties the session store for `session_id`
    pub fn clear_session(&mut self, session_id: &str) {
        if let Some(session) = self.store.get_mut(session_id) {
            session.last_interacted = Instant::now();
            session.data.clear();
            session.data.shrink_to_fit();
        }
        self.try_prune();
    }

    /// removes the session store for `session_id`
    pub fn delete_session(&mut self, session_id: &str) {
        self.store.remove(session_id);
        self.try_prune();
    }

    /// checks if a session store for `session_id` exists
    pub fn session_exists(&mut self, session_id: &str) -> bool {
        self.try_prune();
        self.store.contains_key(session_id)
    }


    /// Accepts a session id, creates a session with it if the ID is not already for an existing
    /// session.
    /// Returns false if the session id was not good or the store already contains the ID
    pub fn add_session(&mut self, session_id: &str) -> bool {
        if session_id_is_good(session_id) && !self.store.contains_key(session_id) {
            let session = Session::new(session_id);
            self.store.insert(session_id.to_string(), session);
            return true;
        }
        false
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
                session_id = self.create_session();
                is_new_session = true;
            }
        } else if !self.session_exists(&session_id) {
            session_id = self.create_session();
            is_new_session = true;
        }
        self.try_prune();
        Some((session_id, is_new_session))
    }

    fn try_prune(&mut self) {
        if self.last_prune.elapsed() < self.prune_duration { return; }
        self.last_prune = Instant::now();

        let mut to_remove = Vec::new();
        for (id, session) in &self.store {
            if session.last_interacted.elapsed() >= self.expiry_duration {
                to_remove.push(id.clone());
            }
        }

        debug_println!("Pruning {} of {} sessions", to_remove.len(), self.store.len());
        for id in to_remove {
            self.store.remove(id.as_str());
        }
        self.store.shrink_to_fit();
    }
}

