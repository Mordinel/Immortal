
use std::collections::HashMap;
use super::{
    request::Request,
    response::Response,
    SessionManagerMtx,
};

pub type Handler = fn(&SessionManagerMtx, &Request, &mut Response);

pub struct Router {
    fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(_sess: &SessionManagerMtx, _req: &Request, res: &mut Response) {
    res.code = "501";
    res.body = b"<h1>501: Not Implemented</h1>".to_vec();
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            fallback: not_implemented,
            routes: HashMap::new(),
        }
    }

    pub fn register(&mut self, method: &str, route: &str, func: Handler) -> bool {
        if !self.routes.contains_key(method) {
            self.routes.insert(method.to_string(), HashMap::new());
        }
        match self.routes.get_mut(method) {
            None => return false,
            Some(inner) => inner,
        }.insert(route.to_string(), func);
        true
    }

    pub fn call(
        &self,
        method: &str,
        req: &Request,
        res: &mut Response,
        session_manager: &SessionManagerMtx) {
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(session_manager, req, res);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(&req.document) {
            None => {
                (self.fallback)(session_manager, req, res);
                return;
            },
            Some(inner) => inner,
        };
        func(session_manager, req, res);
    }
}

