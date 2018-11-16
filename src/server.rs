use crate::network::{socket_handler, Peers};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::prelude::*;

pub(super) fn start_server(port: u16) {
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let listener = TcpListener::bind(&addr).expect(&format!("Could not bind to {}.", addr));
    let peers: Peers = Arc::new(Mutex::new(HashMap::new()));
    let server = listener
        .incoming()
        .for_each(move |socket| socket_handler(socket, peers.clone()))
        .map_err(|err| {
            panic!("Error accepting new socket connection: {}", err);
        });
    tokio::run(server);
}
