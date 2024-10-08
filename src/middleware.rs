
use crate::router::Handler;
use crate::context::Context;

/// Provides middleware functionality
pub struct Middleware {
    middleware: Vec<Handler>,
}

impl Default for Middleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware {
    pub fn new() -> Middleware {
        Self { middleware: Vec::new() }
    }

    /// Inserts a handler into the middleware
    pub fn push(&mut self, func: Handler) {
        self.middleware.push(func);
    }

    /// Runs all the middleware on the `ctx`
    pub fn run(&self, ctx: &mut Context) {
        for func in &self.middleware {
            if ctx.response.is_redirect() {
                return;
            }
            func(ctx);
        }
    }
}
