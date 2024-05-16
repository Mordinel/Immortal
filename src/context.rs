
use uuid::Uuid;

use super::{
    request::Request,
    response::Response,
    session::SessionManager,
};

/// Context is a structure that is exposed to the programmer when registering closures as
/// request handlers.
pub struct Context<'a, 'b> {
    pub request: &'a Request,
    pub response: &'a mut Response<'b>,
    pub session_id: Option<Uuid>,
    session_manager: &'a SessionManager,
}

#[allow(dead_code)]
impl<'a, 'b> Context<'a, 'b> {
    pub fn new(request: &'a Request,
               response: &'a mut Response<'b>, 
               session_id: Option<Uuid>,
               session_manager: &'a SessionManager) -> Self {
        Self {
            request,
            response,
            session_id,
            session_manager,
        }
    }

    /// Makes a write to a session with a key and value
    /// Returns true if a write happened to a session, false if no session id exists
    /// Writing an empty string to this will remove the item from the session storage
    pub fn write_session(&mut self, session_id: &Option<Uuid>, key: &str, value: &str) -> bool {
        if session_id.is_none() {
            return false;
        }
        match self.session_manager.write() {
            Err(_) => false,
            Ok(mut session_manager) => {
                session_manager.write_session(session_id, key, value)
            },
        }
    }

    /// Reads from a session store, the value associated with the key
    /// Returns None if the session or the key is nonexistent
    pub fn read_session(&self, session_id: &Option<Uuid>, key: &str) -> Option<String> {
        if session_id.is_none() {
            return None;
        }
        match self.session_manager.read() {
            Err(_) => None,
            Ok(session_manager) => {
                session_manager.read_session(session_id, key)
            },
        }
    }

    /// Clears the session data of any session ID passed in Shrinks the session data hashmap 
    /// accordingly
    pub fn clear_session(&mut self, session_id: &Option<Uuid>) {
        if session_id.is_none() {
            return;
        }
        match self.session_manager.write() {
            Err(_) => (),
            Ok(mut session_manager) => {
                session_manager.clear_session(session_id)
            },
        }
    }

    /// Completely deletes the session storage related to the passed-in session_id value
    /// Shrinks the session storage hashmap accordingly
    pub fn delete_session(&mut self, session_id: &Option<Uuid>) {
        if session_id.is_none() {
            return;
        }
        match self.session_manager.write() {
            Err(_) => (),
            Ok(mut session_manager) => {
                session_manager.delete_session(session_id)
            },
        }
    }

    // Creates a new session and returns the session id
    pub fn new_session(&mut self) -> Option<Uuid> {
        self.session_manager.write().unwrap().create_session()
    }

    /// Returns true or false if the session associated with session_id exists
    pub fn session_exists(&self, session_id: &Option<Uuid>) -> bool {
        if session_id.is_none() {
            return false;
        }
        match self.session_manager.read() {
            Err(_) => false,
            Ok(session_manager) => {
                session_manager.session_exists(session_id)
            },
        }
    }

    /// Sets the response code and location header
    pub fn redirect(&mut self, location: &str) {
        self.response.code = "302";
        self.response.headers.insert("Location", location.to_string());
    }
}
