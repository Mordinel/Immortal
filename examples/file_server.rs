use std::fs::File;
use std::io::{BufReader, Read};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::RwLock;

use immortal_http::{Immortal, Context};

use lazy_static::lazy_static;
use clap::Parser;

#[derive(Parser)]
#[command()]
struct Cli {
    /// Ip address for the server
    #[arg(short)]
    ip: IpAddr,
    /// Port for the server
    #[arg(short)]
    port: u16,
    /// Directory from which files will be served over HTTP
    #[arg(short)]
    web_root: PathBuf,
}

fn main() {
    let opts = Cli::parse();

    let mut web_root = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        },
    };

    web_root.push(opts.web_root);

    let web_root = match web_root.canonicalize() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error: Unable to canonicalize `{}`: {e}", web_root.display());
            std::process::exit(1);
        }
    };

    *WEB_ROOT.write().unwrap() = web_root;

    let socket_addr = SocketAddr::from((opts.ip, opts.port));

    let mut immortal = Immortal::new();
    immortal.disable_sessions();
    immortal.fallback(web_server);

    if let Err(e) = immortal.listen_with(socket_addr, 4) {
        panic!("Fatal error: {e}");
    }
}

lazy_static! {
    static ref WEB_ROOT: RwLock<PathBuf> = RwLock::new(PathBuf::new());
}

fn web_server(ctx: &mut Context) {
    let mut path = WEB_ROOT.read().unwrap().clone();
    let mut document = ctx.request.document.to_string();
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
                b"wasm" => "application/wasm",
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

