/**
*     Copyright (C) 2022 Mason Soroka-Gill
*
*     This program is free software: you can redistribute it and/or modify
*     it under the terms of the GNU General Public License as published by
*     the Free Software Foundation, either version 3 of the License, or
*     (at your option) any later version.
*
*     This program is distributed in the hope that it will be useful,
*     but WITHOUT ANY WARRANTY; without even the implied warranty of
*     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*     GNU General Public License for more details.
*
*     You should have received a copy of the GNU General Public License
*     along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, ErrorKind};

pub use crate::immortal::request::Request;
pub use crate::immortal::response::Response;
pub use crate::immortal::util::{strip_for_terminal};

pub mod response;
pub mod request;
pub mod util;
pub mod cookie;

#[derive(Debug)]
pub struct Immortal {
    listener: TcpListener,
}

impl Immortal {
    /**
     * Construct a new Immortal server or returns an error
     */
    pub fn new(socket_str: &str) -> Result<Self, String> {
        let listener = match TcpListener::bind(socket_str) {
           Err(e) => return Err(e.to_string()),
           Ok(listener) => listener,
        };
       
        Ok(Self {
            listener,
        })
    }

    /**
     * Listens for incoming connections and sends them to handle_connection
     */
    pub fn listen(&self) -> Result<(), String> {
        match self.listener.local_addr() {
            Err(e) => return Err(e.to_string()),
            Ok(socket) => println!("Server starting at: http://{}", socket),
        };

        for stream in self.listener.incoming() {
            match stream {
                Err(e) => return Err(e.to_string()),
                Ok(stream) => self.handle_connection(stream),
            }
        }
        Ok(())
    }

    /**
     * Reads the TcpStream and handles errors while reading
     */
    fn handle_connection(&self, mut stream: TcpStream) {
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
                    let request = match Request::new(&mut buf) {
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

                    let mut response = Response::new(&request);

                    Self::log(&stream, &request, &response);

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

    fn log(stream: &TcpStream, req: &Request, resp: &Response) {
        let remote_socket = match stream.peer_addr() {
            Err(_) => "<no socket>".to_string(),
            Ok(s) => s.to_string(),
        };
        let date = match resp.header("Date") {
            None => "<no date>",
            Some(thing) => thing,
        };
        let user_agent = match req.header("User-Agent") {
            None => "<no user-agent>",
            Some(thing) => thing,
        };
        println!("{}\t{}\t{}\t{}\t{}\t{}",
                 remote_socket,
                 date,
                 strip_for_terminal(&req.method),
                 resp.code,
                 strip_for_terminal(&req.document),
                 strip_for_terminal(user_agent));
    }
}

