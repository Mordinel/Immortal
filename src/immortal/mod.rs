use std::{
    net::{TcpListener, TcpStream},
    io::{Read, Write, ErrorKind},
    sync::{Arc, Mutex},
    thread, time::Duration,
};

use anyhow::{anyhow, Result};
use scoped_thread_pool::Pool;

pub mod response;
pub mod request;
pub mod middleware;
pub mod router;
pub mod util;
pub mod cookie;
pub mod session;
pub mod context;

pub use crate::immortal::{
    request::Request,
    response::Response,
    middleware::Middleware,
    router::{Router, Handler},
    session::SessionManager,
    context::ImmortalContext,
    util::strip_for_terminal,
};

/// Prints access logs in the same format as Nginx access logs
fn log(stream: &TcpStream, req: &Request, resp: &Response) {
    let remote_socket = match stream.peer_addr() {
        Err(_) => "<no socket>".to_string(),
        Ok(s) => s.ip().to_string(),
    };
    let date = match resp.header("Date") {
        None => "<no date>",
        Some(thing) => thing,
    };
    let user_agent = match req.header("User-Agent") {
        None => "<no user-agent>",
        Some(thing) => thing,
    };
    println!("{}\t{}\t{}\t{}\t{}\t{}\t{}",
             remote_socket,
             date,
             strip_for_terminal(&req.method),
             resp.code,
             resp.body.len(),
             match &req.query.is_empty() {
                 true => strip_for_terminal(&req.document),
                 false => {
                     strip_for_terminal(&req.document) + "?" + &strip_for_terminal(&req.query)
                 }
             },
             strip_for_terminal(user_agent));
}

/// Reads the TcpStream and handles errors while reading
fn handle_connection(
    mut stream: TcpStream,
    session_manager: &SessionManagerMtx,
    middleware: &Middleware,
    router: &Router) {

    let mut buf: [u8; 4096] = [0; 4096];
    loop {
        buf.fill(0u8);
        let read_sz = match stream.read(&mut buf) {
            Err(e) => match e.kind() {
                ErrorKind::Interrupted => {
                    continue;
                },
                _ => { // other errors
                    println!("{}", e);
                    break;
                },
            },
            Ok(sz) => sz,
        };

        match read_sz {
            0 => break,
            _ => {
                let mut request = match Request::new(&buf) {
                    Err(_) => {
                        let mut response = Response::bad();
                        match stream.write(response.serialize().as_slice()) {
                            Err(e) => match e.kind() {
                                ErrorKind::Interrupted => continue,
                                _ => {
                                    eprintln!("{}", e);
                                },
                            },
                            Ok(_) => (),
                        }
                        return;
                    },
                    Ok(req) => req,
                };

                let mut session_id = String::new();
                let mut response = Response::new(&mut request, session_manager, &mut session_id);
                let mut ctx = ImmortalContext::new(&request, &mut response, session_id, session_manager);

                middleware.run(&mut ctx);
                router.call(&request.method, &mut ctx);

                log(&stream, &request, &response);

                if let Err(e) = stream.write(response.serialize().as_slice()) {
                    if e.kind() == ErrorKind::Interrupted { continue }
                };
            },
        };
    };
}

pub type SessionManagerMtx = Arc<Mutex<SessionManager>>;

/// Immortal middleware and routing configuration, as well as the session manager.
#[derive(Default)]
pub struct Immortal {
    middleware: Middleware,
    router: Router,
    session_manager: SessionManagerMtx,
}

#[allow(dead_code)]
impl Immortal {
    /// Construct a new Immortal server
    pub fn new() -> Self {
        Self {
            middleware: Middleware::new(),
            router: Router::new(),
            session_manager: Arc::new(Mutex::new(SessionManager::default())),
        }
    }

    /// Listens for incoming connections, with as many threads as the system has available for
    /// parallelism
    pub fn listen(&self, socket_str: &str) -> Result<()> {
        self.listen_with(socket_str, thread::available_parallelism()?.get())
    }

    /// Listens for incoming connections using a specific amount of threads
    pub fn listen_with(&self, socket_str: &str, thread_count: usize) -> Result<()> {
        let listener = TcpListener::bind(socket_str)?;

        match listener.local_addr() {
            Err(e) => return Err(anyhow!(e)),
            Ok(socket) => println!("Server starting at: http://{}", socket),
        };

        let thread_pool = Pool::new(thread_count);

        thread_pool.scoped(|scope| {
            for stream in listener.incoming() {
                match stream {
                    Err(e) => return Err(anyhow!(e)),
                    Ok(stream) => scope.execute(|| {
                        handle_connection(stream, &self.session_manager, &self.middleware, &self.router)
                    }),
                };
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn process_buffer(&mut self, request_buffer: &[u8]) -> Vec<u8> {
        let mut request = match Request::new(request_buffer) {
            Err(_) => return Response::bad().serialize(),
            Ok(req) => req,
        };

        let mut session_id = String::new();
        let mut response = Response::new(&mut request, &self.session_manager, &mut session_id);
        let mut ctx = ImmortalContext::new(&request, &mut response, session_id, &self.session_manager);

        self.middleware.run(&mut ctx);
        self.router.call(&request.method, &mut ctx);

        response.serialize()
    }

    /// Adds middleware that gets executed just before the router.
    ///
    /// if a middleware handler produces a redirect, all of the following middleware handlers are
    /// skipped and the redirect is yielded, if middleware produces a redirect, the router is
    /// bypassed and custom routes do not run. 
    pub fn add_middleware(&mut self, func: Handler) {
        self.middleware.push(func);
    }

    /// Calls into the router to register a function
    /// Returns true if a route was registered
    pub fn register(&mut self, method: &str, route: &str, func: Handler) -> bool {
        self.router.register(method, route, func)
    }

    /// Calls into the router to unregister a function
    /// Returns true if a route was unregistered
    pub fn unregister(&mut self, method: &str, route: &str) -> bool {
        self.router.unregister(method, route)
    }

    /// Registers the fallback function for no method/route match requests, or for if your
    /// implementation handles this.
    pub fn fallback(&mut self, func: Handler) {
        self.router.fallback = func;
    }

    /// Sets the server-side session expiry duration
    pub fn set_expiry_duration(&mut self, duration: Duration) {
        self.session_manager.lock().unwrap().set_expiry_duration(duration);
    }

    /// Sets the server-side session prune duration
    pub fn set_prune_duration(&mut self, duration: Duration) {
        self.session_manager.lock().unwrap().set_prune_duration(duration);
    }
}

