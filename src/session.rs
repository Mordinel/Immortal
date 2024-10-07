use std::{
    collections::HashMap,
    time::{Instant, Duration},
    sync::{Arc, RwLock},
};

use uuid::Uuid;

use super::request::Cookies;

use debug_print::debug_eprintln;

pub type SessionManager = Arc<RwLock<InternalSessionManager>>;

#[allow(dead_code)]
pub struct Session {
    id: Uuid,
    data: HashMap<String, String>,
    last_mutated: Instant,
}

impl Session {
    fn new(id: Uuid) -> Session {
        Session {
            id,
            data: HashMap::new(),
            last_mutated: Instant::now(),
        }
    }
}

pub type SessionStore = HashMap<Uuid, Session>;

/// provides APIs to interact with user session stores.
pub struct InternalSessionManager {
    is_enabled: bool,
    store: SessionStore,
    expiry_duration: Duration,
    prune_duration: Duration,
    last_prune: Instant,
}

impl Default for InternalSessionManager {
    fn default() -> Self {
        InternalSessionManager::new(Duration::from_secs(60*60), Duration::from_secs(60*10))
    }
}

impl InternalSessionManager {
    pub fn new(expiry_duration: Duration, prune_duration: Duration) -> InternalSessionManager {
        InternalSessionManager {
            is_enabled: true,
            store: HashMap::new(),
            expiry_duration,
            prune_duration,
            last_prune: Instant::now(),
        }
    }


    /// returns true if sessions are enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// enables sessions
    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    /// disables sessions and clears all existing sessions
    pub fn disable(&mut self) {
        self.is_enabled = false;
        self.store.clear();
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
    pub fn generate_id() -> Uuid {
        uuid::Uuid::new_v4()
    }

    /// generates a new session and returns the ID
    pub fn create_session(&mut self) -> Option<Uuid> {
        if !self.is_enabled() {
            return None;
        }
        let mut out;
        loop {
            out = Self::generate_id();
            if !self.store.contains_key(&out) {
                break;
            }
        }
        self.store.insert(out, Session::new(out));
        self.try_prune();
        Some(out)
    }

    /// writes `value` to the key-value session store as `key` for the `session_id` session store.
    pub fn write_session(&mut self, session_id: &Option<Uuid>, key: &str, value: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_none() {
            return false;
        }
        if let Some(session) = self.store.get_mut(&session_id.unwrap()) {
            session.last_mutated = Instant::now();
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
    pub fn read_session(&self, session_id: &Option<Uuid>, key: &str) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        if session_id.is_none() {
            return None;
        }
        if let Some(session) = self.store.get(&session_id.unwrap()) {
            if let Some(value) = session.data.get(key) {
                return Some(value.to_owned());
            }
        }
        None
    }

    /// empties the session store for `session_id`
    pub fn clear_session(&mut self, session_id: &Option<Uuid>) {
        if !self.is_enabled() {
            return;
        }
        if session_id.is_none() {
            return;
        }
        if let Some(session) = self.store.get_mut(&session_id.unwrap()) {
            session.last_mutated = Instant::now();
            session.data.clear();
            session.data.shrink_to_fit();
        }
        self.try_prune();
    }

    /// removes the session store for `session_id`
    pub fn delete_session(&mut self, session_id: &Option<Uuid>) {
        if !self.is_enabled() {
            return;
        }
        if session_id.is_none() {
            return;
        }
        self.store.remove(&session_id.unwrap());
        self.try_prune();
    }

    /// checks if a session store for `session_id` exists
    pub fn session_exists(&self, session_id: &Option<Uuid>) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_none() {
            return false;
        }
        self.store.contains_key(&session_id.unwrap())
    }

    /// Accepts a session id, creates a session with it if the ID is not already for an existing
    /// session.
    /// Returns false if the session id was not good or the store already contains the ID
    pub fn add_session(&mut self, session_id: &Option<Uuid>) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_none() {
            return false;
        }
        if !self.store.contains_key(&session_id.unwrap()) {
            if let Some(id) = session_id {
                let session = Session::new(id.clone());
                self.store.insert(*id, session);
                return true;
            }
        }
        false
    }

    /// tries to get an existing session 
    /// if a session does not exist, a session is created.
    /// The returned tuple contains the session id and a boolean, the boolean is true if the
    /// created session is new, if it is false, the session id is for an existing session.
    pub fn get_or_create_session(&mut self, cookies: &Cookies) -> Option<(Uuid, bool)>{
        if !self.is_enabled() {
            return None;
        }
        let mut session_id = None;

        let mut is_new_session = false;
        if cookies.contains_key("id") {
            session_id = cookies.get("id")
                .map(|id| id.value.parse::<Uuid>().ok()).flatten();
            if !self.session_exists(&session_id) {
                session_id = self.create_session();
                is_new_session = true;
            }
        } else if !self.session_exists(&session_id) {
            session_id = self.create_session();
            is_new_session = true;
        }
        self.try_prune();
        if session_id.is_some() {
            Some((session_id.unwrap(), is_new_session))
        } else {
            None
        }
    }

    fn try_prune(&mut self) {
        if !self.is_enabled() {
            return;
        }
        if self.last_prune.elapsed() < self.prune_duration { return; }
        self.last_prune = Instant::now();

        let mut to_remove = Vec::new();
        for (id, session) in &self.store {
            if session.last_mutated.elapsed() >= self.expiry_duration {
                to_remove.push(id.clone());
            }
        }

        debug_eprintln!("Pruning {} of {} sessions", to_remove.len(), self.store.len());
        for id in to_remove {
            self.store.remove(&id);
        }
        self.store.shrink_to_fit();
    }
}

