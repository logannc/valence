extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate bytes;
extern crate futures;
extern crate tokio;
extern crate uuid;
extern crate rand;
// extern crate tokio_codec;
// extern crate tokio_serde_bincode;

mod client;
mod network;
mod server;
mod types;

use clap::{App, Arg};

fn main() {
    let matches = App::new("Simple Chat Application")
                        .version("0.1")
                        .author("Logan Collins <logan@lcspace.net>")
                        .about("Simple Demo Rust Chat Server/Client")
                        .arg(Arg::with_name("listen")
                                .short("l")
                                .long("listen")
                                .value_name("PORT")
                                .takes_value(true)
                                .help("Start the application in server mode, specifying which port to listen to. If set, ignores <SERVER>."))
                        .arg(Arg::with_name("server")
                                .value_name("SERVER")
                                .help("The server address:port to connect to.")
                                .required_unless("listen"))
                        .get_matches();
    if let Some(port) = matches.value_of("listen") {
        self::server::start_server(port.parse().expect("Expected an integer port number."));
    } else if let Some(addr) = matches.value_of("server") {
        self::client::start_client(addr.parse().expect("Expected an IP:PORT argument."));
    } else {
        // `server` should have been required by clap if `listen` wasn't provided.
        unreachable!();
    }
}
