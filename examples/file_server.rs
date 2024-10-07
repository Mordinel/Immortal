use std::fs::File;
use std::io::{BufReader, Read};

use immortal_http::{Immortal, Context};

fn main() {
    let mut immortal = Immortal::new();
    immortal.disable_sessions();
    immortal.fallback(web_server);

    if let Err(e) = immortal.listen_with("127.0.0.1:7777", 4) {
        panic!("{}", e);
    }
}

fn web_server(ctx: &mut Context) {
    let mut document = ctx.request.document.clone();
    document = collapse_chr(&document, '/');
    document = collapse_chr(&document, '.');

    if document == "/" || document.contains("../") {
        document = "/index.html".to_string();
    } else if document == "" {
        four_oh_four(ctx);
        return;
    } else if document.len() > 255 {
        ctx.response.code = "414";
        ctx.response.status = "URI TOO LONG";
        ctx.response.body.extend(b"<h1>414: Uri Too Long!</h1>".iter());
        return;
    }

    let mut path = match std::env::current_dir() {
        Ok(path) => path,
        Err(why) => {
            eprintln!("ERROR: {why}");
            return;
        },
    };
    path.push("webroot");
    path.push(document[1..].to_string());

    if path.exists() && path.is_file() {
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(why) => {
                eprintln!("ERROR: {why}");
                four_oh_four(ctx);
                return;
            }
        };
        let mut file = BufReader::new(file);
        match file.read_to_end(&mut ctx.response.body) {
            Ok(_) => (),
            Err(why) => {
                eprintln!("ERROR: {why}");
                ctx.response.body.clear();
                four_oh_four(ctx);
                return;
            },
        }
        ctx.response.headers.insert("Content-Type", match path.extension() {
            Some(ext) => match ext.as_encoded_bytes() {
                b"html" => "text/html",
                b"css" => "text/css",
                b"js" => "application/javascript",
                b"txt" => "text/plain",
                b"jpg" => "image/jpeg",
                b"png" => "image/png",
                b"gif" => "image/gif",
                b"ico" => "image/x-icon",
                b"zip" => "application/zip",
                b"pdf" => "application/pdf",
                b"woff2" => "font/woff2",
                b"woff" => "font/woff",
                _ => "application/octet-stream",
            },
            None => "application/octet-stream",
        }.to_string());
    } else {
        four_oh_four(ctx);
        return;
    }
}

fn four_oh_four(ctx: &mut Context) {
    ctx.response.code = "404";
    ctx.response.body.extend(b"<h1>404: File Not Found!</h1>".iter());
}

fn collapse_chr(s: &str, chr: char) -> String {
    let mut new_string = String::with_capacity(s.len());
    let mut last_was_chr = false;
    for c in s.chars() {
        if c == chr {
            last_was_chr = true;
        } else {
            if last_was_chr {
                new_string.push(chr);
            }
            last_was_chr = false;
            new_string.push(c);
        }
    }
    if last_was_chr {
        new_string.push(chr);
    }
    new_string
}

