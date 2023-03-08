#![feature(iter_intersperse)]
use crate::immortal::Immortal;
mod immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let mut immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };

    immortal.register("GET", "/", |_sess, _req, res| {
        res.code = "200";
        res.body = b"<h1>200: Ok</h1>".to_vec();
    });

    immortal.register("GET", "/favicon.ico", |_sess, _req, res| {
        res.code = "404";
        res.body = b"<h1>404: Not found</h1>".to_vec();
    });

    immortal.register("GET", "/teapot", |_sess, _req, res| {
        res.code = "418";
        res.body = b"<h1>418: I am a little teapot</h1>".to_vec();
    });

    immortal.register("GET", "/hello", |sess, req, res| {
        res.code = "200";
        res.headers.insert("Content-Type", "text/plain;charset=UTF-8".to_string());
        let name = req.get("name").unwrap_or_default();
        if name.is_empty() {
            // getting a cookie named "id"
            match req.cookies.get("id") {
                None => {
                    res.body = b"Pass your name into the `name` GET parameter!".to_vec();
                },
                Some(cookie) => {
                    // accessing the session storage with that id value for a "name" key
                    match sess.lock().unwrap().read_session(&cookie.value, "name") {
                        None => {
                            res.body = b"Pass your name into the `name` GET parameter!".to_vec();
                        },
                        Some(name) => {
                            // if it exists, welcome the user back
                            res.body = format!("Welcome back {name}!").into_bytes();
                        }
                    }
                }
            }
        } else {
            res.body = format!("Hello {name}!").into_bytes();
            match req.cookies.get("id") {
                None => {},
                Some(cookie) => {
                    sess.lock().unwrap().write_session(&cookie.value, "name", name);
                }
            };
        }
    });

    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
