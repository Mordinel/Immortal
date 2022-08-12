use std::net::{TcpListener, TcpStream};
use std::io::{Read, ErrorKind};

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

    pub fn listen(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Err(e) => println!("{:?}", e),
                Ok(stream) => self.handle_connection(stream),
            }
        }
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let mut buf: [u8; 4096] = [0; 4096];
        loop {
            buf.iter_mut().for_each(|b| *b = 0);
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
                _ => println!("{}", String::from_utf8_lossy(&buf)),
            };
        };
    }
}

