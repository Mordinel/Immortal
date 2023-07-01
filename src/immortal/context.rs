
use super::{
    request::Request,
    response::Response,
    SessionManagerMtx,
};

/// ImmortalContext is a structure that is exposed to the programmer when registering closures as
/// request handlers.
pub struct ImmortalContext<'a, 'b> {
    pub request: &'a Request,
    pub response: &'a mut Response<'b>,
    session_manager: &'a SessionManagerMtx,
}

#[allow(dead_code)]
impl<'a, 'b> ImmortalContext<'a, 'b> {
    pub fn new(request: &'a Request,
               response: &'a mut Response<'b>, 
               session_manager: &'a SessionManagerMtx) -> Self {
        Self {
            request,
            response,
            session_manager,
        }
    }

    /// Makes a write to a session with a key and value
    /// Returns true if a write happened to a session, false if no session id exists
    /// Writing an empty string to this will remove the item from the session storage
    pub fn write_session(&mut self, session_id: &str, key: &str, value: &str) -> bool {
        match self.session_manager.lock() {
            Err(_) => false,
            Ok(mut session_manager) => {
                session_manager.write_session(session_id, key, value)
            },
        }
    }

    /// Reads from a session store, the value associated with the key
    /// Returns None if the session or the key is nonexistent
    pub fn read_session(&self, session_id: &str, key: &str) -> Option<String> {
        match self.session_manager.lock() {
            Err(_) => None,
            Ok(session_manager) => {
                session_manager.read_session(session_id, key)
            },
        }
    }

    /// Clears the session data of any session ID passed in Shrinks the session data hashmap 
    /// accordingly
    pub fn clear_session(&mut self, session_id: &str) {
        match self.session_manager.lock() {
            Err(_) => (),
            Ok(mut session_manager) => {
                session_manager.clear_session(session_id)
            },
        }
    }

    /// Completely deletes the session storage related to the passed-in session_id value
    /// Shrinks the session storage hashmap accordingly
    pub fn delete_session(&mut self, session_id: &str) {
        match self.session_manager.lock() {
            Err(_) => (),
            Ok(mut session_manager) => {
                session_manager.delete_session(session_id)
            },
        }
    }

    /// Returns true or false if the session associated with session_id exists
    pub fn session_exists(&self, session_id: &str) -> bool {
        match self.session_manager.lock() {
            Err(_) => false,
            Ok(session_manager) => {
                session_manager.session_exists(session_id)
            },
        }
    }

    pub fn redirect(&mut self, location: &str) {
        self.response.code = "302";
        self.response.headers.insert("Location", location.to_string());
    }
}
