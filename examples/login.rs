use std::time::Duration;

use immortal_http::{
    Immortal,
    context::Context,
    cookie::Cookie,
    util::escape_html,
};

fn main() {
    let mut immortal = Immortal::new();

    immortal.set_session_duration(Duration::from_secs(1800));
    immortal.set_inactive_duration(Duration::from_secs(600));
    immortal.set_prune_rate(Duration::from_secs(60));
    immortal.enable_sessions();

    immortal.fallback(|ctx| {
        ctx.response_mut().code = "404";
    });

    immortal.add_middleware(|ctx| {
        ctx.response_mut().headers.insert("X-Frame-Options", "deny".to_string());
        ctx.response_mut().headers.insert("X-Content-Type-Options", "nosniff".to_string());
        ctx.response_mut().headers.insert("Referrer-Policy", "no-referrer".to_string());
    });

    immortal.add_middleware(|ctx| {
        match ctx.request().document {
            "/" | "/login" | "/logout" | "/favicon.ico" => return,
            _ => {},
        }

        if !is_logged_in(ctx) {
            set_message(ctx, "Must log in to access resources");
            ctx.redirect("/login");
        }
    });

    immortal.register("GET", "/", |ctx| {
        match get_username(ctx) {
            None=> {
                ctx.response_mut().body.append(&mut b"<h1>Welcome to the website!</h1>".to_vec());
                ctx.response_mut().body.append(&mut b"<p>Click <a href=\"/login\">HERE</a> to go to the login page</p>".to_vec());
            },
            Some(username) => {
                let username = escape_html(&username);
                ctx.response_mut().body.append(&mut format!("<h1>Welcome to the website, {username}!</h1>").as_bytes().to_vec());
                ctx.response_mut().body.append(&mut b"<p>Click <a href=\"/logout\">HERE</a> to log out</p>".to_vec());
                ctx.response_mut().body.append(&mut b"<p>Click <a href=\"/secret\">HERE</a> to see the secret</p>".to_vec());
            },
        };
    });

    immortal.register("GET", "/login", |ctx| {
        if is_logged_in(ctx) {
            ctx.redirect("/");
            return;
        }

        ctx.response_mut().body.append(&mut b"
<form action=\"/login\" method=\"post\">
<label for=\"username\">Username: </label>
<input type=\"text\" id=\"username\" name=\"username\" required></input><br>
<label for=\"password\">Password: </label>
<input type=\"password\" id=\"password\" name=\"password\" required></input>
<input type=\"submit\" value=\"Submit\">
</form>".to_vec()
        );

        match get_message(ctx) {
            None => {},
            Some(message) => {
                ctx.response_mut().body.append(
                    &mut format!("<br><p>{}</p>", escape_html(&message)).as_bytes().to_vec()
                );
                clear_message(ctx);
            },
        };
    });

    immortal.register("POST", "/login", |ctx| {
        if is_logged_in(ctx) {
            ctx.redirect("/");
            return;
        }

        if ctx.request_mut().post("username").is_some() && ctx.request_mut().post("password").is_some() {
            let username = ctx.request_mut().post("username").unwrap().to_string();
            let password = ctx.request_mut().post("password").unwrap().to_string();
            if /*username == "admin" &&*/ password == "lemon42" { // could do a DB lookup here
                log_in(ctx, &username);
                ctx.redirect("/");
                return;
            }
        }

        set_message(ctx, "Failed to log in");
        ctx.redirect("/login");
    });

    immortal.register("GET", "/logout", |ctx| {
        if is_logged_in(ctx) {
            log_out(ctx);
            set_message(ctx, "Logged out");
        } else {
            set_message(ctx, "Not logged in");
        }
        ctx.redirect("/login");
    });

    immortal.register("GET", "/secret", |ctx| {
        ctx.response_mut().body.append("<h1>This is the super secret page</h1>".as_bytes().to_vec().as_mut());
    });

    if let Err(e) = immortal.listen(([127, 0, 0, 1], 7777)) {
        panic!("{}", e);
    }
}

fn get_username(ctx: &mut Context) -> Option<String> {
    ctx.read_session(ctx.session_id, "username")
}

fn log_out(ctx: &mut Context) {
    ctx.write_session(ctx.session_id, "username", "");
}

fn log_in(ctx: &mut Context, username: &str) {
    ctx.delete_session(ctx.session_id);

    let session_id = ctx.new_session();
    if !session_id.is_nil() {
        ctx.response_mut().cookies.push(
            Cookie::builder().name("id").value(&session_id.to_string()).http_only(true).build()
            );
        ctx.write_session(session_id, "username", username);
        ctx.session_id = session_id;
    }
}

fn get_message(ctx: &mut Context) -> Option<String> {
    ctx.read_session(ctx.session_id, "message")
}

fn set_message(ctx: &mut Context, message: &str) {
    ctx.write_session(ctx.session_id, "message", message);
}

fn clear_message(ctx: &mut Context) {
    set_message(ctx, "");
}

fn is_logged_in(ctx: &mut Context) -> bool {
    get_username(ctx).is_some()
}

