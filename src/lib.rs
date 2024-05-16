#![feature(iter_intersperse)]
pub mod context;
pub mod cookie;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod session;
pub mod util;

use request::Request;
use response::Response;
use middleware::Middleware;
use router::{Router, Handler};
use session::{InternalSessionManager, SessionManager};
use context::Context;
use util::{strip_for_terminal, code_color};

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    time::Duration,
    thread,
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::*;
use debug_print::debug_println;

fn log(stream: &TcpStream, req: &Request, resp: &Response, sent: usize) {
    let remote_socket = match stream.peer_addr() {
        Err(_) => "<no socket>".red().bold(),
        Ok(s) => s.ip().to_string().normal(),
    };

    let now = Utc::now();
    let date_time = now.format("%a, %d %b %Y %H:%M:%S").to_string();
    let time_stamp = format!("[{:<17}]",
                             format!("{}.{}",
                                now.timestamp(),
                                now.timestamp_subsec_micros())
                            .bright_blue());

    let method = match req.method.as_str() {
        "" => "<no method>".red().bold(),
        _ => strip_for_terminal(&req.method).normal(),
    };

    let document = match req.document.as_str() {
        "" => "<no document>".red().bold(),
        _ => strip_for_terminal(&req.document).normal(),
    };

    let user_agent = match req.header("User-Agent") {
        None => "<no user-agent>".red().bold(),
        Some(thing) => strip_for_terminal(thing).normal(),
    };

    println!("{}  {}  {}  {}\t{}  {}\t{}\t{}",
             date_time,
             time_stamp,
             remote_socket,
             method,
             code_color(resp.code),
             sent,
             if req.query.is_empty() {
                document
             } else {
                format!("{}?{}", document, &strip_for_terminal(&req.query)).normal()
             },
             user_agent);
}

/// Reads the TcpStream and handles errors while reading
fn handle_connection(
    mut stream: TcpStream,
    session_manager: &SessionManager,
    middleware: &Middleware,
    router: &Router
) {
    let peer_addr = match stream.peer_addr() {
        Ok(addr) => Some(addr),
        Err(_) => None,
    };
    let mut buf: [u8; 4096] = [0; 4096];
    let read_sz = match stream.read(&mut buf) {
        Err(e) => match e.kind() { _ => {
            debug_println!("{}", e);
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return;
        }, },
        Ok(sz) => sz,
    };
    debug_println!("SERVER <<< {read_sz} <<< {}", peer_addr.unwrap());

    match read_sz {
        //0 => break,
        0 => {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return
        },
        _ => {
            let mut request = match Request::new(&buf, peer_addr.as_ref()) {
                Err(_) => {
                    let request = Request::bad();
                    let mut response = Response::bad();
                    match stream.write(response.serialize().as_slice()) {
                        Ok(sent) => {
                            log(&stream, &request, &response, sent);
                            debug_println!("SERVER >>> {sent} >>> {}", peer_addr.unwrap());
                        },
                        Err(_) => {
                            log(&stream, &request, &response, 0);
                            debug_println!("SERVER >>> SEND ERROR >>> !");
                        },
                    }
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                },
                Ok(req) => req,
            };

            let mut session_id = None;
            let mut response = Response::new(&mut request, session_manager, &mut session_id);
            let mut ctx = Context::new(&request, &mut response, session_id, session_manager);

            middleware.run(&mut ctx);
            router.call(&request.method, &mut ctx);

            match stream.write(response.serialize().as_slice()) {
                Ok(sent) => {
                    log(&stream, &request, &response, sent);
                    debug_println!("SERVER >>> {sent} >>> {}", peer_addr.unwrap());
                },
                Err(_) => {
                    log(&stream, &request, &response, 0);
                    println!("SERVER >>> SEND ERROR >>> !");
                },
            };
        },
    };
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// Immortal middleware and routing configuration, as well as the session manager.
#[derive(Default)]
pub struct Immortal {
    middleware: Middleware,
    router: Router,
    session_manager: SessionManager,
}

#[allow(dead_code)]
impl Immortal {
    /// Construct a new Immortal server
    pub fn new() -> Self {
        Self {
            middleware: Middleware::new(),
            router: Router::new(),
            session_manager: Arc::new(RwLock::new(InternalSessionManager::default())),
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

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()?;

        let _ = thread_pool.scope(|scope| {
            for stream in listener.incoming() {
                match stream {
                    Err(e) => return Err(anyhow!(e)),
                    Ok(stream) => scope.spawn(|_s| {
                        handle_connection(stream, &self.session_manager, &self.middleware, &self.router);
                    }),
                };
            };
            Ok(())
        });
        Ok(())
    }

    /// Pass a buffer through the HTTP implementation without listening on a port or dispatching
    /// tasks to threads.
    pub fn process_buffer(&mut self, request_buffer: &[u8]) -> Vec<u8> {
        let mut request = match Request::from_slice(request_buffer) {
            Err(_) => return Response::bad().serialize(),
            Ok(req) => req,
        };

        let mut session_id = None;
        let mut response = Response::new(&mut request, &self.session_manager, &mut session_id);
        let mut ctx = Context::new(&request, &mut response, session_id, &self.session_manager);

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
        self.session_manager.write().unwrap().set_expiry_duration(duration);
    }

    /// Sets the server-side session prune duration
    pub fn set_prune_duration(&mut self, duration: Duration) {
        self.session_manager.write().unwrap().set_prune_duration(duration);
    }

    /// configures sessions to be disabled, clears existing server-side sessions
    pub fn disable_sessions(&mut self) {
        self.session_manager.write().unwrap().disable();
    }

    /// configures sessions to be enabled
    pub fn enable_sessions(&mut self) {
        self.session_manager.write().unwrap().enable();
    }
}

