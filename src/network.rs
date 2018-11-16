use bincode::{serialize, deserialize};
use crate::types::{ClientMessage, ServerMessage, ClientName};
use tokio::prelude::*;
use tokio::net::TcpStream;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;
use bytes::{Bytes, BytesMut};
use futures::Sink;
use tokio::io::{ReadHalf, WriteHalf};
use serde::Serialize;

pub struct Peer {
    writer: FramedWrite<WriteHalf<TcpStream>, LengthDelimitedCodec>,
    nickname: ClientName,
}
pub type Peers = Arc<Mutex<HashMap<Uuid, Peer>>>;

type FramedReader = FramedRead<ReadHalf<TcpStream>, LengthDelimitedCodec>;
type FramedWriter = FramedWrite<WriteHalf<TcpStream>, LengthDelimitedCodec>;

fn start_send_message<'a, T, I>(msg: T, peers: I) where T: Serialize, I: Iterator<Item=&'a mut Peer>{
    let serialized_msg: Bytes = serialize(&msg).unwrap().into();
    for peer in peers {
        peer.writer.start_send(serialized_msg.clone());
    }
}

fn start_send_all<T>(msg: T, peers: &mut Peers) where T: Serialize {
    start_send_message(msg, peers.lock().unwrap().values_mut());
}

fn flush_peers(peers: &mut Peers) {
    for peer in peers.lock().unwrap().values_mut() {
        peer.writer.poll_complete(); 
    }
}

fn handshake_handler(buf: BytesMut, framed_reader: FramedReader, framed_writer: FramedWriter, mut peers: Peers) -> Result<(Uuid, FramedReader), Error> {
    match deserialize(&*buf) {
        Ok(cm) => match cm {
            // Client joined with a nickname...
            ClientMessage::Join(name) => {
                let ID = Uuid::new_v4();
                start_send_all(ServerMessage::Joined(name.clone()), &mut peers);
                flush_peers(&mut peers);
                peers.lock().unwrap().insert(ID, Peer{ writer: framed_writer, nickname: name});
                Ok((ID, framed_reader))
            }
            // Client sent some other kind of message first. Error out.
            _ => {
                Err(Error::from(ErrorKind::InvalidData))
            },
        },
        Err(_) => Err(Error::from(ErrorKind::InvalidData))
    }
}

fn handle_client_message(msg: ClientMessage, ID: Uuid, peers: &mut Peers) -> Result<(), Error> {
    match msg {
        ClientMessage::Message(msg) => {
            let msg = ServerMessage::Message(msg, peers.lock().unwrap()[&ID].nickname.clone());
            start_send_message(msg, peers.lock().unwrap().iter_mut().filter(|(&k, v)| k != ID).map(|(_, v)| v));
            Ok(())
        },
        ClientMessage::Nickname(name) => {
            let old_nickname = peers.lock().unwrap()[&ID].nickname.clone();
            let name_clone = name.clone();
            peers.lock().unwrap().entry(ID).and_modify(|peer|{
                peer.nickname = name_clone;
            });
            let msg = ServerMessage::ServerText(format!("{} changed their name to {}.", old_nickname, name));
            start_send_all(msg, peers);
            Ok(())
        },
        ClientMessage::Quit => {
            // let peer = peers.lock().unwrap().remove(&ID).unwrap();
            // let msg = ServerMessage::Quit(peer.nickname);
            // start_send_all(msg, &mut peers);
            Err(Error::from(ErrorKind::ConnectionAborted))
        },
        ClientMessage::Join(name) => Ok(()), // Ignore this misbehaving message.
    }
}

fn handle_frame(frame: BytesMut, ID: Uuid, mut peers: Peers) -> Result<(), Error> {
    let result = match deserialize(&*frame) {
        Ok(msg) => handle_client_message(msg, ID, &mut peers),
        Err(err) => Err(Error::from(ErrorKind::InvalidData)), 
    };
    for peer in peers.lock().unwrap().values_mut() {
        peer.writer.poll_complete();
    }
    result
}

fn handle_disconnect(ID: Uuid, mut peers: Peers) -> Result<(), Error> {
    let peer = peers.lock().unwrap().remove(&ID).unwrap();
    let msg = ServerMessage::Quit(peer.nickname);
    start_send_all(msg, &mut peers);
    for peer in peers.lock().unwrap().values_mut() {
        peer.writer.poll_complete();
    }
    Ok(())
}

fn spawn_message_handler(input: (Uuid, FramedReader), peers: Peers) -> Result<(), Error> {
    let (ID, framed_reader) = input;
    let peers_clone = peers.clone();
    let handler = framed_reader
                    .for_each(move|frame| { handle_frame(frame, ID, peers.clone()) })
                    .and_then(move|_| handle_disconnect(ID, peers_clone))
                    .or_else(|err| { panic!("error: {}", err); Ok(()) });
    tokio::spawn(handler);
    Ok(())
}

pub fn socket_handler(socket: TcpStream, peers: Peers) -> Result<(), Error> {
    let (reader, writer) = socket.split();
    let framed_reader = FramedRead::new(reader, LengthDelimitedCodec::new());
    let framed_writer = FramedWrite::new(writer, LengthDelimitedCodec::new());
    let peers_clone = peers.clone();
    let frame_handler = framed_reader
        // We want to deal with just the first frame,
        // making sure it is a `ClientMessage::Join`
        .into_future()
        .map_err(|(err, _)| panic!("wat: {}", err))
        .and_then(|(maybe_buf, framed_reader)| {
            println!("got first bytes");
            maybe_buf.map_or_else(
                // I don't think this can happen because the `FramedRead` wouldn't yield?
                || Err(Error::from(ErrorKind::UnexpectedEof)),
                |buf| handshake_handler(buf, framed_reader, framed_writer, peers),
            )
        })
        .and_then(|input|{
            spawn_message_handler(input, peers_clone)
        }).map_err(|err| panic!("annother server error: {}", err));
    tokio::spawn(frame_handler);
    Ok(())
}