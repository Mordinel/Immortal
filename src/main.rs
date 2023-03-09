#![feature(iter_intersperse)]
use crate::immortal::Immortal;
mod immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let mut immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };

    immortal.fallback(|ctx| {
        ctx.response.code = "404";
    });

    immortal.register("GET", "/", |ctx| {
        match ctx.read_session(&ctx.request.session_id, "username") {
            None=> {
                ctx.response.body.append(&mut b"<h1>Welcome to the website!</h1>".to_vec());
                ctx.response.body.append(&mut b"<p>Click <a href=\"/login\">HERE</a> to go to the login page</p>".to_vec());
            },
            Some(username) => {
                ctx.response.body.append(&mut format!("<h1>Welcome to the website, {username}!</h1>").as_bytes().to_vec());
                ctx.response.body.append(&mut b"<p>Click <a href=\"/logout\">HERE</a> to log out</p>".to_vec());
            },
        };
    });

    immortal.register("GET", "/login", |ctx| {
        if ctx.read_session(&ctx.request.session_id, "username").is_some() {
            ctx.response.code = "302";
            ctx.response.headers.insert("Location", String::from("/"));
            return;
        }

        ctx.response.body.append(&mut b"
<form action=\"/login\" method=\"post\">
<label for=\"username\">Username: </label>
<input type=\"text\" id=\"username\" name=\"username\" required></input><br>
<label for=\"password\">Password: </label>
<input type=\"password\" id=\"password\" name=\"password\" required></input>
<input type=\"submit\" value=\"Submit\">
</form>".to_vec()
        );

        match ctx.read_session(&ctx.request.session_id, "message") {
            None => {},
            Some(message) => {
                ctx.response.body.append(
                    &mut format!("<br><p>{}</p>", ctx.html_escape(&message)).as_bytes().to_vec()
                );
                ctx.write_session(&ctx.request.session_id, "message", "");
            },
        };
    });

    immortal.register("POST", "/login", |ctx| {
        ctx.response.code = "302";
        if ctx.read_session(&ctx.request.session_id, "username").is_some() {
            ctx.response.headers.insert("Location", "/".to_string());
            return;
        }

        if ctx.request.post("username").is_some() && ctx.request.post("password").is_some() {
            let username = ctx.request.post("username").unwrap();
            let password = ctx.request.post("password").unwrap();
            if /*username == "admin" &&*/ password == "lemon42" {
                ctx.write_session(&ctx.request.session_id, "username", username);
                ctx.response.headers.insert("Location", "/".to_string());
                return;
            }
        }

        ctx.write_session(&ctx.request.session_id, "message", "Failed to log in");
        ctx.response.headers.insert("Location", "/login".to_string());
    });

    immortal.register("GET", "/logout", |ctx| {
        ctx.response.code = "302";
        ctx.response.headers.insert("Location", "/login".to_string());

        if ctx.read_session(&ctx.request.session_id, "username").is_some() {
            ctx.write_session(&ctx.request.session_id, "username", "");
            ctx.write_session(&ctx.request.session_id, "message", "Logged out");
        } else {
            ctx.write_session(&ctx.request.session_id, "message", "Not logged in");
        }
    });

    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
