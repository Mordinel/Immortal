
use std::collections::HashMap;
use crate::immortal::request::Request;
use crate::immortal::response::Response;

pub type Handler = fn(&Request, &mut Response);

pub struct Router {
    fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(_req: &Request, res: &mut Response) {
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

    pub fn call(&self, method: &str, req: &Request, res: &mut Response) {
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(req, res);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(&req.document) {
            None => {
                (self.fallback)(req, res);
                return;
            },
            Some(inner) => inner,
        };
        func(req, res);
    }
}

