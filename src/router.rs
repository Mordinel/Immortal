
use std::collections::HashMap;

use super::{
    context::Context, 
    util::is_redirect,
};

pub type Handler = fn(&mut Context);

/// provides an API to register and lookup HTTP routes
pub struct Router {
    pub fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(ctx: &mut Context) {
    eprintln!("ERROR: default fallback handler fired, you probably mean to replace this.");
    ctx.response.code = "501";
    ctx.response.body = b"<h1>501: Not Implemented</h1>".to_vec();
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Creates a new router
    pub fn new() -> Self {
        Self {
            fallback: not_implemented,
            routes: HashMap::new(),
        }
    }

    /// register a path with a function callback
    /// if a request document path matches the callback path, the callback is fired.
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

    /// removes a registered path
    pub fn unregister(&mut self, method: &str, route: &str) -> bool {
        match self.routes.get_mut(method) {
            None => false,
            Some(inner) => {
                inner.remove(route).is_some()
            },
        }
    }

    /// tries to call a registered path
    /// if it fails, the fallback is automatically called.
    /// if response is already a redirect, don't call.
    pub fn call(&self, method: &str, ctx: &mut Context) {
        if is_redirect(ctx.response) {
            return;
        }
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(ctx);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(&ctx.request.document) {
            None => {
                (self.fallback)(ctx);
                return;
            },
            Some(inner) => inner,
        };
        func(ctx);
    }
}

