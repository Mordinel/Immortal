use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use atomic_time::{AtomicDuration, AtomicInstant};
use dashmap::DashMap;
use debug_print::{debug_eprintln, debug_println};
use uuid::Uuid;

#[cfg(feature = "threading")]
use rayon::prelude::*;

use crate::cookie::Cookie;

pub struct Session {
    data: HashMap<String, String>,
    created: Instant,
    last_mutated: Instant,
    last_accessed: Instant,
}

impl Session {
    fn new() -> Session {
        let now = Instant::now();
        Session {
            data: HashMap::new(),
            created: now,
            last_mutated: now,
            last_accessed: now.into(),
        }
    }
}

pub fn session_prune_task(
    session_manager: Arc<SessionManager>,
    stop: Arc<AtomicBool>,
) {
    loop {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        session_manager.prune();
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// provides APIs to interact with user session stores.
pub struct SessionManager {
    is_enabled: AtomicBool,
    store: DashMap<Uuid, Session>,
    /// The maximum duration that a session may be allowed to persist for
    /// regardless of inactivity.
    session_duration: AtomicDuration,
    /// The duration a session will persist for if inactive.
    inactive_duration: AtomicDuration,
    /// How often the session store is pruned
    prune_rate: AtomicDuration,
    /// When the list was last pruned
    last_prune: AtomicInstant,
}

impl Default for SessionManager {
    fn default() -> Self {
        SessionManager::new(
            Duration::from_secs(12 * 3600), // 12 hours session duration
            Duration::from_secs(3600),      //  1 hour  inactive duration
            Duration::from_secs(60)         //  1 min   prune rate
        )
    }
}

impl SessionManager {
    pub fn new(
        session_duration: Duration,
        inactive_duration: Duration,
        prune_rate: Duration,
    ) -> SessionManager {
        SessionManager {
            is_enabled: AtomicBool::new(false),
            store: DashMap::new(),
            session_duration: AtomicDuration::new(session_duration),
            inactive_duration: AtomicDuration::new(inactive_duration),
            prune_rate: AtomicDuration::new(prune_rate),
            last_prune: AtomicInstant::now(),
        }
    }

    /// returns true if sessions are enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.load(Relaxed)
    }

    /// enables sessions
    pub fn enable(&self) {
        self.is_enabled.store(true, Relaxed);
    }

    /// disables sessions and clears all existing sessions
    pub fn disable(&self) {
        self.is_enabled.store(false, Relaxed);
        self.store.clear();
    }

    /// sets the maximum duration that a session may be allowed to persist for 
    /// regardless of inactivity
    pub fn set_session_duration(&self, duration: Duration) {
        self.session_duration.store(duration, Relaxed);
    }

    /// sets the expiry duration for sessions
    pub fn set_inactive_duration(&self, duration: Duration) {
        self.inactive_duration.store(duration, Relaxed);
    }

    /// sets the prune rate for sessions, will attempt to prune old sessions every `duration`
    pub fn set_prune_rate(&self, duration: Duration) {
        self.prune_rate.store(duration, Relaxed);
    }

    /// generates a new session id without storing a session
    pub fn generate_id() -> Uuid {
        uuid::Uuid::new_v4()
    }

    /// generates a new session and returns the ID
    pub fn create_session(&self) -> Uuid {
        if !self.is_enabled() {
            return Uuid::nil();
        }
        let mut out;
        loop {
            out = Self::generate_id();
            if !self.store.contains_key(&out) {
                break;
            }
        }
        self.store.insert(out, Session::new());

        #[cfg(not(feature = "threading"))]
        self.prune();
        out
    }

    /// writes `value` to the key-value session store as `key` for the `session_id` session store.
    pub fn write_session(&self, session_id: Uuid, key: &str, value: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_nil() {
            #[cfg(not(feature = "threading"))]
            self.prune();
            return false;
        }
        if let Some(mut session) = self.store.get_mut(&session_id) {
            let now = Instant::now();
            session.last_mutated = now;
            session.last_accessed = now.into();
            if value.is_empty() {
                session.data.remove(key);
                session.data.shrink_to_fit();
            } else {
                session.data.insert(key.to_owned(), value.to_owned());
            }
            return true;
        }
        #[cfg(not(feature = "threading"))]
        self.prune();
        false
    }

    /// reads the value associated with `key` for the `session_id` session store.
    pub fn read_session(&self, session_id: Uuid, key: &str) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        if session_id.is_nil() {
            return None;
        }
        if let Some(mut session) = self.store.get_mut(&session_id) {
            session.last_accessed = Instant::now();
            if let Some(value) = session.data.get(key) {
                return Some(value.to_owned());
            }
        }
        None
    }

    /// empties the session store for `session_id`
    pub fn clear_session(&self, session_id: Uuid) {
        if !self.is_enabled() {
            return;
        }
        if session_id.is_nil() {
            #[cfg(not(feature = "threading"))]
            self.prune();
            return;
        }
        if let Some(mut session) = self.store.get_mut(&session_id) {
            let now = Instant::now();
            session.last_mutated = now;
            session.last_accessed = now.into();
            session.data.clear();
            session.data.shrink_to_fit();
        } else {
            #[cfg(not(feature = "threading"))]
            self.prune();
        }
    }

    /// removes the session store for `session_id`
    pub fn delete_session(&self, session_id: Uuid) {
        if !self.is_enabled() {
            return;
        }
        if session_id.is_nil() {
            return;
        }
        self.store.remove(&session_id);
        #[cfg(not(feature = "threading"))]
        self.prune();
    }

    /// checks if a session store for `session_id` exists
    pub fn session_exists(&self, session_id: Uuid) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_nil() {
            return false;
        }
        self.store.contains_key(&session_id)
    }

    /// Accepts a session id, creates a session with it if the ID is not already for an existing
    /// session.
    /// Returns false if the session id was not good or the store already contains the ID
    pub fn add_session(&self, session_id: Uuid) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if session_id.is_nil() {
            #[cfg(not(feature = "threading"))]
            self.prune();
            return false;
        }
        if !self.store.contains_key(&session_id) {
            let session = Session::new();
            self.store.insert(session_id, session);
            return true;
        }
        #[cfg(not(feature = "threading"))]
        self.prune();
        false
    }

    /// tries to get an existing session 
    /// if a session does not exist, a session is created.
    /// The returned tuple contains the session id and a boolean, the boolean is true if the
    /// created session is new, if it is false, the session id is for an existing session.
    pub fn get_or_create_session(&self, cookies: &HashMap<String, Cookie>) -> Option<(Uuid, bool)>{
        if !self.is_enabled() {
            return None;
        }
        let mut session_id = Uuid::nil();

        let mut is_new_session = false;
        if cookies.contains_key("id") {
            session_id = cookies.get("id")
                .map(|id| id.value.parse::<Uuid>().ok())
                .flatten()
                .unwrap_or(Uuid::nil());
            if !self.session_exists(session_id) {
                session_id = self.create_session();
                is_new_session = true;
            }
        } else if !self.session_exists(session_id) {
            session_id = self.create_session();
            is_new_session = true;
        }
        if !session_id.is_nil() {
            Some((session_id, is_new_session))
        } else {
            #[cfg(not(feature = "threading"))]
            self.prune();
            None
        }
    }

    pub fn prune(&self) {
        #[cfg(not(feature = "threading"))]
        if !self.is_enabled() {
            return;
        }

        if self.last_prune.load(Relaxed).elapsed() < self.prune_rate.load(Relaxed) { return; }
        self.last_prune.store(Instant::now(), Relaxed);

        #[cfg(feature = "threading")]
        {
            let to_remove = self.store.par_iter()
                .filter(|pair| {
                    if pair.last_accessed.elapsed() >= self.inactive_duration.load(Relaxed) {
                        return true;
                    } else if pair.created.elapsed() >= self.session_duration.load(Relaxed) {
                        return true;
                    }
                    return false;
                }).map(|pair| {
                    pair.pair().0.clone()
                }).collect::<Vec<Uuid>>();
            let total = self.store.len();
            to_remove.par_iter().for_each(|id| { self.store.remove(id); });
            debug_eprintln!("Pruned {}/{} sessions.", to_remove.len(), total);
        }

        #[cfg(not(feature = "threading"))]
        {
            let mut to_remove = Vec::new();
            for session in self.store.iter() {
                if session.value().last_accessed.elapsed() >= self.inactive_duration.load(Relaxed) {
                    to_remove.push(session.key().clone());
                } else if session.created.elapsed() >= self.session_duration.load(Relaxed) {
                    to_remove.push(session.key().clone());
                }
            }
            let total = self.store.len();
            for id in &to_remove {
                self.store.remove(&id);
            }
            debug_eprintln!("Pruned {}/{} sessions.", to_remove.len(), total);
        }
        self.store.shrink_to_fit();
    }
}

