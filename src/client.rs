use bincode::{serialize, deserialize};
use crate::types::{ClientMessage, ServerMessage};
use futures::Future;
use std::net::SocketAddr;
use tokio::codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::net::TcpStream;
use tokio::prelude::*;
use rand::random;

pub(super) fn start_client(addr: SocketAddr) {
    let client = TcpStream::connect(&addr)
        .and_then(|socket| {
            let (reader, writer) = socket.split();
            let mut framed_reader = FramedRead::new(reader, LengthDelimitedCodec::new());
            let mut writer = FramedWrite::new(writer, LengthDelimitedCodec::new());
            let name = format!("Jeorge{}", random::<u8>());
            println!("Sending...");
            writer.start_send(
                serialize(&ClientMessage::Join(name))
                    .unwrap()
                    .into(),
            );
            writer.start_send(
                serialize(&ClientMessage::Message("Test Message!!".into()))
                    .unwrap()
                    .into(),
            );
            writer.start_send(
                serialize(&ClientMessage::Message("Test Message!!".into()))
                    .unwrap()
                    .into(),
            );
            println!("Waiting for send to complete...");
            writer.poll_complete();
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
            println!("Waiting on replies...");
            tokio::spawn(handler);
            // let ts = writer.send();
            // ts.wait().unwrap();
            // // framed_writer.start_send();
            // // framed_writer.start_send(serialize(&ClientMessage::Message("Test Chat Message!".into())).expect("failed to serialize").into());
            // // framed_writer.poll_complete().expect("Failed to send");
            // println!("WE shold be done sending...");
            // // serializer.start_send(ClientMessage::Join("Jeorge".into())).expect("Joining channel failed");
            // // serializer.start_send(ClientMessage::Message("Test Message!".into())).expect("Sending first message failed.");
            // // serializer.poll_complete().expect("Failed to send.");
            // // let deserializer = FramedRead::new(reader, SMCodec{});
            Ok(())
        })
        .map_err(|err| {
            panic!("ERROR establishing connection: {}", err);
        });
    tokio::run(client);
}
