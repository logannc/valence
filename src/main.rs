extern crate tokio;
extern crate clap;

use clap::{Arg, App};

mod server;
mod client;

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
        self::client::start_client(addr.into());
    } else {
        // SERVER should have been required by clap if `listen` wasn't provided.
        unreachable!();
    }
}
