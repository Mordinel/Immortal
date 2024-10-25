
use std::collections::HashMap;

use crate::context::Context;

pub type Handler = fn(&mut Context);

/// provides an API to register and lookup HTTP routes
pub struct Router {
    pub fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(ctx: &mut Context) {
    eprintln!("ERROR: default fallback handler fired, you probably mean to replace this.");
    ctx.response_mut().code = "501";
    ctx.response_mut().body = b"<h1>501: Not Implemented</h1>".to_vec();
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
    pub fn call(&self, ctx: &mut Context) {
        let method = ctx.request().method;
        let document = ctx.request().document;

        if ctx.response().is_redirect() {
            return;
        }
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(ctx);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(document) {
            None => {
                (self.fallback)(ctx);
                return;
            },
            Some(inner) => inner,
        };
        func(ctx);
    }
}

