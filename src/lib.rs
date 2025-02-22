#![feature(iter_intersperse)]

use std::cell::RefCell;
use std::fmt::Display;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::error;
use std::thread::{self, JoinHandle};
use std::sync::Arc;

pub mod context;
pub mod cookie;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod session;
pub mod util;

pub use request::Request;
use request::RequestError;
pub use response::Response;
pub use context::Context;
use middleware::Middleware;
use router::{Router, Handler};
use session::SessionManager;
use util::{strip_for_terminal, code_color};
use uuid::Uuid;

use chrono::Utc;
use colored::*;
use debug_print::debug_eprintln;

#[derive(Debug)]
pub enum ImmortalError<'a> {
    /// Io: std::io::Error
    Io(io::Error),
    /// Rayon: Thread Pool Build Error
    #[cfg(feature = "threading")]
    Tpbe(rayon::ThreadPoolBuildError),
    /// TCP accept() failed
    AcceptError(io::Error),
    RequestError(RequestError<'a>),
}

impl Display for ImmortalError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for ImmortalError<'_> {}

#[inline]
fn log(stream: &TcpStream, req: Rc<RefCell<Request>>, resp: Rc<RefCell<Response>>, sent: usize) {
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

    let method = match req.borrow().method {
        "" => "<no method>".red().bold(),
        _ => strip_for_terminal(req.borrow().method).normal(),
    };

    let document = match req.borrow().document {
        "" => "<no document>".red().bold(),
        _ => strip_for_terminal(req.borrow().document).normal(),
    };

    let user_agent = match req.borrow_mut().header("User-Agent") {
        None => "<no user-agent>".red().bold(),
        Some(thing) => strip_for_terminal(thing).normal(),
    };

    println!("{}  {}  {}  {}\t{}  {}\t{}\t{}",
             date_time,
             time_stamp,
             remote_socket,
             method,
             code_color(resp.borrow().code),
             sent,
             if req.borrow_mut().query_raw.is_empty() {
                document
             } else {
                format!("{}?{}", document, &strip_for_terminal(req.borrow_mut().query_raw)).normal()
             },
             user_agent);
}

#[inline]
fn stream_write(stream: &mut TcpStream, request: Rc<RefCell<Request>>, response: Rc<RefCell<Response>>) {
    let data = response.borrow_mut().serialize();
    match stream.write(data.as_slice()) {
        Ok(sent) => {
            log(stream, request, response, sent);
        },
        Err(_) => {
            log(stream, request, response, 0);
        },
    }
}

/// Reads the TcpStream and handles errors while reading
fn handle_connection(
    mut stream: TcpStream,
    session_manager: Arc<SessionManager>,
    middleware: &Middleware,
    router: &Router
) {
    let peer_addr = match stream.peer_addr() {
        Ok(addr) => Some(addr),
        Err(_) => None,
    };
    let mut buf: [u8; 4096] = [0; 4096];
    let read_sz = match stream.read(&mut buf) {
        Err(_e) => {
            debug_eprintln!("{}", _e);
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return;
        },
        Ok(sz) => sz,
    };

    match read_sz {
        //0 => break,
        0 => {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return
        },
        _ => {
            let request = match Request::new(&buf, peer_addr.as_ref()) {
                Err(RequestError::ProtoVersionInvalid(_)) => {
                    let request = Rc::new(RefCell::new(Request::bad()));
                    let response = Rc::new(RefCell::new(Response::bad()));
                    response.borrow_mut().code = "505";
                    stream_write(&mut stream, request, response);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                },
                Err(_) => {
                    let request = Rc::new(RefCell::new(Request::bad()));
                    let response = Rc::new(RefCell::new(Response::bad()));
                    stream_write(&mut stream, request, response);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                },
                Ok(req) => req,
            };

            let request_rc = Rc::new(RefCell::new(request));
            let mut session_id = Uuid::nil();
            let response = Response::new(request_rc.clone(), session_manager.clone(), &mut session_id);
            let response_rc = Rc::new(RefCell::new(response));
            let mut ctx = Context::new(request_rc.clone(), response_rc.clone(), session_id, session_manager.clone());

            middleware.run(&mut ctx);
            router.call(&mut ctx);

            stream_write(&mut stream, request_rc, response_rc);
        },
    };
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// Immortal middleware and routing configuration, as well as the session manager.
#[derive(Default)]
pub struct Immortal {
    middleware: Middleware,
    router: Router,
    session_manager: Arc<SessionManager>,
    #[allow(dead_code)]
    session_prune_task: Option<(JoinHandle<()>, Arc<AtomicBool>)>,
}

#[allow(dead_code)]
impl Immortal {
    /// Construct a new Immortal server
    pub fn new() -> Self {
        Self {
            middleware: Middleware::new(),
            router: Router::new(),
            session_manager: Arc::new(SessionManager::default()),
            session_prune_task: None,
        }
    }

    /// Listens for incoming connections, with as many threads as the system has available for
    /// parallelism
    pub fn listen<S>(
        &self,
        socket_addr: S
    ) -> Result<(), ImmortalError> where S: Into<SocketAddr> {
        self.listen_with(
            socket_addr,
            thread::available_parallelism()
                .map_err(ImmortalError::Io)?
                .get(),
        )
    }

    /// Listens for incoming connections using a specific amount of threads
    ///
    /// If `threading` feature is not present, `thread_count` will be ignored
    pub fn listen_with<S>(
        &self,
        socket_addr: S,
        #[allow(unused_variables)]
        thread_count: usize
    ) -> Result<(), ImmortalError> where S: Into<SocketAddr> {
        let socket_addr: SocketAddr = socket_addr.into();
        let listener = TcpListener::bind(socket_addr)
            .map_err(ImmortalError::Io)?;

        println!("Server starting at: http://{socket_addr}");

        #[cfg(feature = "threading")] 
        {
            let thread_pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .map_err(ImmortalError::Tpbe)?;

            let _ = thread_pool.scope(|scope| -> Result<(), ImmortalError> {

                loop {
                    let (stream, _peer_addr) = listener.accept()
                        .map_err(ImmortalError::AcceptError)?;

                    scope.spawn(|_s| {
                        handle_connection(
                            stream,
                            self.session_manager.clone(),
                            &self.middleware,
                            &self.router,
                        );
                    });
                }
            });
        }

        #[cfg(not(feature = "threading"))]
        loop {
            let (stream, _peer_addr) = listener.accept()
                .map_err(ImmortalError::AcceptError)?;

            handle_connection(
                stream,
                self.session_manager.clone(),
                &self.middleware,
                &self.router,
            );
        }

        #[cfg(feature = "threading")] 
        Ok(())
    }

    /// Pass a buffer through the HTTP implementation without listening on a port or dispatching
    /// tasks to threads.
    pub fn process_buffer(&mut self, request_buffer: &[u8]) -> Vec<u8> {
        let request = match Request::from_slice(request_buffer) {
            Err(_) => return Response::bad().serialize(),
            Ok(req) => req,
        };

        let request_rc = Rc::new(RefCell::new(request));
        let mut session_id = Uuid::nil();
        let response = Response::new(request_rc.clone(), self.session_manager.clone(), &mut session_id);
        let response_rc = Rc::new(RefCell::new(response));
        let mut ctx = Context::new(request_rc.clone(), response_rc.clone(), session_id, self.session_manager.clone());

        self.middleware.run(&mut ctx);
        self.router.call(&mut ctx);

        let data = response_rc.borrow_mut().serialize();
        data
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

    /// Registers the fallback function for when a request is not caught by the router
    /// or for if you want to handle all requests manually
    pub fn fallback(&mut self, func: Handler) {
        self.router.fallback = func;
    }

    /// sets the maximum duration that a session may be allowed to persist for
    /// regardless of inactivity
    pub fn set_session_duration(&self, duration: Duration) {
        self.session_manager.set_session_duration(duration);
    }

    /// Sets the server-side session inactivity expiry duration
    pub fn set_inactive_duration(&self, duration: Duration) {
        self.session_manager.set_inactive_duration(duration);
    }

    /// Sets the prune rate for sessions, will attempt to prune old sessions every `duration`
    pub fn set_prune_rate(&self, duration: Duration) {
        self.session_manager.set_prune_rate(duration);
    }

    /// Configures sessions to be disabled, clears existing server-side sessions
    pub fn disable_sessions(&mut self) {
        self.session_manager.disable();

        #[cfg(feature = "threading")]
        if let Some((handle, stop)) = self.session_prune_task.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            handle.join().unwrap();
        }
    }

    /// Configures sessions to be enabled
    pub fn enable_sessions(&mut self) {
        self.session_manager.enable();

        #[cfg(feature = "threading")]
        {
            let stop = Arc::new(AtomicBool::new(false));
            let stop_clone = stop.clone();
            let session_manager_clone = self.session_manager.clone();

            let thread_handle = thread::spawn(|| {
                session::session_prune_task(
                    session_manager_clone,
                    stop_clone,
                );
            });

            self.session_prune_task = Some((thread_handle, stop));
        }
    }
}

impl Drop for Immortal {
    fn drop(&mut self) {
        self.disable_sessions();
    }
}

