use std::{
    net::{TcpListener, TcpStream},
    io::{Read, Write, ErrorKind},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, Result};
use scoped_thread_pool::Pool;

pub mod response;
pub mod request;
pub mod router;
pub mod util;
pub mod cookie;
pub mod session;
pub mod context;

pub use crate::immortal::{
    request::Request,
    response::Response,
    router::{
        Router,
        Handler,
    },
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
fn handle_connection(mut stream: TcpStream, session_manager: &SessionManagerMtx, router: &Router) {
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
                let mut request = match Request::new(&mut buf) {
                    Err(_) => {
                        let mut response = Response::bad();
                        match stream.write(response.serialize().as_slice()) {
                            Err(e) => match e.kind() {
                                ErrorKind::Interrupted => continue,
                                _ => {
                                    println!("{}", e);
                                    break;
                                },
                            },
                            Ok(_) => break,
                        }
                    },
                    Ok(req) => req,
                };

                let mut response = Response::new(&mut request, session_manager);

                let ctx = ImmortalContext::new(&request, &mut response, session_manager);
                router.call(&request.method, ctx);

                log(&stream, &request, &response);

                if let Err(e) = stream.write(response.serialize().as_slice()) {
                    if e.kind() == ErrorKind::Interrupted { continue }
                };

                if request.keep_alive {
                    continue;
                } else {
                    break;
                }
            },
        };
    };
}

pub type SessionManagerMtx = Arc<Mutex<SessionManager>>;

/// Immortal provides the socket and threading configuration, as well as the routing API to program
/// against.
pub struct Immortal {
    listener: TcpListener,
    thread_pool: Pool,
    router: Router,
    session_manager: SessionManagerMtx,
}

#[allow(dead_code)]
impl Immortal {
    /// Construct a new Immortal server or returns an error
    pub fn new(socket_str: &str) -> Result<Self> {
        let listener = TcpListener::bind(socket_str)?;

        Ok(Self {
            listener,
            thread_pool: Pool::new(thread::available_parallelism()?.get()),
            router: Router::new(),
            session_manager: Arc::new(Mutex::new(SessionManager::new())),
        })
    }

    /// Listens for incoming connections and sends them to handle_connection
    pub fn listen(&self) -> Result<()> {
        match self.listener.local_addr() {
            Err(e) => return Err(anyhow!(e)),
            Ok(socket) => println!("Server starting at: http://{}", socket),
        };

        self.thread_pool.scoped(|scope| {
            for stream in self.listener.incoming() {
                match stream {
                    Err(e) => return Err(anyhow!(e)),
                    Ok(stream) => scope.execute(|| {
                        handle_connection(stream, &self.session_manager, &self.router)
                    }),
                };
            }
            Ok(())
        })?;

        Ok(())
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
}

