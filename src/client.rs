use bincode::{serialize, deserialize};
use crate::types::{ClientMessage, ServerMessage};
use futures::Future;
use std::net::SocketAddr;
use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::io::ReadHalf;

type FramedReader = FramedRead<ReadHalf<TcpStream>, LengthDelimitedCodec>;

fn spawn_frame_handler(framed_reader: FramedReader) {
    let handler = framed_reader.for_each(|frame|{
        match deserialize::<ServerMessage>(&*frame) {
            Ok(sm) => {
                println!("{:?}", sm)
            },
            Err(err) => {
                println!("err: {}", err)
            },
        }
        Ok(())
    }).map_err(|err|{
        panic!("error: {}", err);
    });
    tokio::spawn(handler);
}

pub(super) fn start_client(addr: SocketAddr, nickname: String) {
    let client = TcpStream::connect(&addr)
        .and_then(|socket| {
            let (reader, writer) = socket.split();
            let writer = FramedWrite::new(writer, LengthDelimitedCodec::new());
            let framed_reader = FramedRead::new(reader, LengthDelimitedCodec::new());
            spawn_frame_handler(framed_reader);
            let join_msg = serialize(&ClientMessage::Join(nickname.into()))
                    .unwrap()
                    .into();
            println!("Waiting to join server...");
            let mut writer = writer.send(join_msg).wait().unwrap();
            println!("You have now joined the server!");
            loop {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf);
                let msg = serialize(&ClientMessage::Message(buf)).unwrap().into();
                writer = writer.send(msg).wait().unwrap();
            }
            Ok(())
        })
        .map_err(|err| {
            panic!("ERROR establishing connection: {}", err);
        });
    tokio::run(client);
}
