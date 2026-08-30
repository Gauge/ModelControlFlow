//! The companion site: it receives what people choose to publish, and shows it.
//!
//! **It is not MCF, and that is deliberate.** MCF cannot send: there is no
//! destination in it, no address to configure, and a check that holds that
//! absence against the tree. So a contribution arrives here because a person
//! moved a file, which is the act MCF's own terms describe. Nothing here talks
//! back to anybody's machine.
//!
//! **It is its own workspace** for the same reason. MCF refuses to listen on a
//! network; this does nothing else. Whatever the site grows to need must never
//! become a condition of a measurement somebody takes at home.

mod archive;
mod http;
mod page;

use std::net::TcpListener;

fn main() {
    let address = std::env::var("MCF_SITE_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port: u16 = std::env::var("MCF_SITE_PORT")
        .ok()
        .and_then(|held| held.parse().ok())
        .unwrap_or(8080);
    let root = std::env::var("MCF_SITE_ARCHIVE")
        .unwrap_or_else(|_| "./contributions".to_owned());
    let archive = archive::Archive::at(&root);

    let listener = match TcpListener::bind((address.as_str(), port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("mcf-site: {address}:{port} could not be bound: {error}");
            std::process::exit(1);
        }
    };
    println!("mcf-site is on http://{address}:{port}");
    println!("  keeping contributions in {root}");
    println!("  {} held", archive.all().len());

    for stream in listener.incoming().flatten() {
        handle(&stream, &archive);
    }
}

fn handle(stream: &std::net::TcpStream, archive: &archive::Archive) {
    let Ok(incoming) = http::read(stream) else {
        http::respond(stream, 400, "text/plain; charset=utf-8", b"bad request\n");
        return;
    };
    match (incoming.method.as_str(), incoming.path.as_str()) {
        ("GET", "/") => http::respond(
            stream,
            200,
            "text/html; charset=utf-8",
            page::index(&archive.all()).as_bytes(),
        ),
        ("POST", "/contribute") => match archive.keep(&incoming.body) {
            Ok(digest) => http::respond(
                stream,
                200,
                "text/html; charset=utf-8",
                page::kept(&digest).as_bytes(),
            ),
            Err(why) => http::respond(
                stream,
                400,
                "text/html; charset=utf-8",
                page::refused(&why).as_bytes(),
            ),
        },
        ("GET", path) if path.starts_with("/c/") => {
            let name = path.trim_start_matches("/c/");
            match archive.one(name) {
                Some(held) => http::respond(
                    stream,
                    200,
                    "text/html; charset=utf-8",
                    page::one(&held).as_bytes(),
                ),
                None => http::respond(
                    stream,
                    404,
                    "text/html; charset=utf-8",
                    page::refused("there is nothing here by that name").as_bytes(),
                ),
            }
        }
        ("GET", _) => http::respond(
            stream,
            404,
            "text/html; charset=utf-8",
            page::refused("there is nothing here by that name").as_bytes(),
        ),
        _ => http::respond(
            stream,
            405,
            "text/plain; charset=utf-8",
            b"that is not something this asks for\n",
        ),
    }
}
