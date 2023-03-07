use crate::immortal::Immortal;
mod immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let mut immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };

    immortal.register("GET", "/", |_req, res| {
        res.code = "200";
        res.body = b"<h1>200: Ok</h1>".to_vec();
    });

    immortal.register("GET", "/favicon.ico", |_req, res| {
        res.code = "404";
        res.body = b"<h1>404: Not found</h1>".to_vec();
    });

    immortal.register("GET", "/teapot", |_req, res| {
        res.code = "418";
        res.body = b"<h1>418: I am a little teapot</h1>".to_vec();
    });

    immortal.register("GET", "/hello", |req, res| {
        res.code = "200";
        res.headers.insert("Content-Type", "text/plain;charset=UTF-8".to_string());
        let name = req.get("name").unwrap_or_default();
        if name.is_empty() {
            res.body = b"Pass your name into the `name` GET parameter!".to_vec();
        } else {
            res.body = format!("Hello {name}!").into_bytes();
        }
    });

    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
