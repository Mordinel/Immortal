
use std::collections::HashMap;
use super::ImmortalContext;

pub type Handler = fn(&mut ImmortalContext);

pub struct Router {
    pub fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(ctx: &mut ImmortalContext) {
    ctx.response.code = "501";
    ctx.response.body = b"<h1>501: Not Implemented</h1>".to_vec();
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

    pub fn unregister(&mut self, method: &str, route: &str) -> bool {
        match self.routes.get_mut(method) {
            None => return false,
            Some(inner) => {
                match inner.remove(route) {
                    None => return false,
                    Some(_) => return true,
                };
            },
        };
    }

    pub fn call(
        &self,
        method: &str,
        mut ctx: ImmortalContext) {
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(&mut ctx);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(&ctx.request.document) {
            None => {
                (self.fallback)(&mut ctx);
                return;
            },
            Some(inner) => inner,
        };
        func(&mut ctx);
    }
}

