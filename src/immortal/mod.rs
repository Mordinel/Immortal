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
use std::io::{Read, ErrorKind};
use crate::immortal::request::Request;
mod request;

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
           Err(e) => return Err(format!("{:?}", e)),
           Ok(listener) => listener,
        };
       
        Ok(Self {
            listener: listener,
        })
    }

    /**
     * Listens for incoming connections and sends them to handle_connection
     */
    pub fn listen(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Err(e) => println!("{:?}", e),
                Ok(stream) => self.handle_connection(stream),
            }
        }
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
                    _ => {
                        println!("{:?}", e);
                        break;
                    },
                },
                Ok(sz) => sz,
            };

            match read_sz {
                0 => break,
                _ => {
                    let _request = match Request::new(&mut buf) {
                        Err(_) => break,
                        Ok(req) => req,
                    };
                },
            };
        };
    }
}

