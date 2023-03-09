#![feature(iter_intersperse)]
use crate::immortal::Immortal;
mod immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let mut immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };

    immortal.register("GET", "/", |ctx| {
        ctx.response.code = "200";
        ctx.response.body = b"<h1>200: Ok</h1>".to_vec();
    });

    immortal.register("GET", "/favicon.ico", |ctx| {
        ctx.response.code = "404";
        ctx.response.body = b"<h1>404: Not found</h1>".to_vec();
    });

    immortal.register("GET", "/teapot", |ctx| {
        ctx.response.code = "418";
        ctx.response.body = b"<h1>418: I am a little teapot</h1>".to_vec();
    });

    immortal.register("GET", "/hello", |ctx| {
        ctx.response.code = "200";
        ctx.response.headers.insert("Content-Type", "text/plain;charset=UTF-8".to_string());
        let name = ctx.request.get("name").unwrap_or_default();
        if name.is_empty() {
            // getting a cookie named "id"
            match ctx.request.cookies.get("id") {
                None => {
                    ctx.response.body = b"Pass your name into the `name` GET parameter!".to_vec();
                },
                Some(cookie) => {
                    // accessing the session storage with that id value for a "name" key
                    match ctx.read_session(&cookie.value, "name") {
                        None => {
                            ctx.response.body = b"Pass your name into the `name` GET parameter!".to_vec();
                        },
                        Some(name) => {
                            // if it exists, welcome the user back
                            ctx.response.body = format!("Welcome back {name}!").into_bytes();
                        },
                    }
                },
            }
        } else {
            ctx.response.body = format!("Hello {name}!").into_bytes();
            match ctx.request.cookies.get("id") {
                None => {},
                Some(cookie) => {
                    ctx.write_session(&cookie.value, "name", name);
                }
            };
        }
    });

    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
