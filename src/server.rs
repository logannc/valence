use bincode::{serialize, deserialize};
use crate::types::{ClientMessage, ServerMessage, ClientName};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Write};
use std::sync::{Arc, Mutex};
use tokio::codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::net::TcpListener;
use tokio::prelude::*;
// use rand::random;
use uuid::Uuid;
use bytes::Bytes;

struct Peer<T> {
    writer: FramedWrite<T, LengthDelimitedCodec>,
    nickname: ClientName,
}

pub(super) fn start_server(port: u16) {
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let listener = TcpListener::bind(&addr).expect(&format!("Could not bind to {}.", addr));
    // UUID -> (FramedWriter, Nickname) TODO: make struct
    let peers: Arc<Mutex<HashMap<Uuid, Peer<_>>>> = Arc::new(Mutex::new(HashMap::new()));
    let peers_clone = peers.clone();
    let server = listener
        .incoming()
        .for_each(move |socket| {
            let (reader, writer) = socket.split();
            let framed_reader = FramedRead::new(reader, LengthDelimitedCodec::new());
            let framed_writer = FramedWrite::new(writer, LengthDelimitedCodec::new());
            let mut peers_clone_one = peers_clone.clone();
            let mut peers_clone_two = peers_clone.clone();
            let mut peers_clone_three = peers_clone.clone();
            let mut peers_clone_four = peers_clone.clone();
            let frame_handler = framed_reader
                // We want to deal with just the first frame,
                // making sure it is a `ClientMessage::Join`
                .into_future()
                .map_err(|(err, _)| panic!("wat: {}", err))
                .and_then(move|(maybe_buf, framed_reader)| {
                    println!("got first bytes");
                    maybe_buf.map_or_else(
                        // I don't think this can happen because the `FramedRead` wouldn't yield?
                        || Err(Error::from(ErrorKind::UnexpectedEof)),
                        move |buf| match deserialize(&*buf) {
                            Ok(cm) => match cm {
                                // Client joined with a nick name...
                                ClientMessage::Join(name) => {
                                    let ID = Uuid::new_v4();
                                    let peers = peers_clone_one;
                                    // let peers = Arc::get_mut(&mut peers_clone_one).expect("Why no hashmap?");
                                    let serialized_joined: Bytes = serialize(&ServerMessage::Joined(name.clone())).unwrap().into();
                                    for peer in peers.lock().unwrap().values_mut() {
                                        peer.writer.start_send(serialized_joined.clone()); 
                                    }
                                    // TODO: maybe remove this flush loop later...
                                    for peer in peers.lock().unwrap().values_mut() {
                                        peer.writer.poll_complete(); 
                                    }
                                    peers.lock().unwrap().insert(ID, Peer{ writer: framed_writer, nickname: name});
                                    Ok((ID, framed_reader))
                                }
                                // Client sent some other kind of message first. Error out.
                                _ => {
                                    Err(Error::from(ErrorKind::InvalidData)) // TODO: revisit relevant errors
                                },
                            },
                            Err(_) => Err(Error::from(ErrorKind::InvalidData)) // TODO: revisit relevant errors
                        },
                    )
                })
                .and_then(|(ID, framed_reader)|{
                    let peers = peers_clone_two;
                    let h = framed_reader.for_each(move|frame|{
                        println!("Got another frame...");
                        let ret = match deserialize(&*frame) {
                            Ok(cm) => match cm {
                                ClientMessage::Message(msg) => {
                                    let serialized_msg: Bytes = serialize(&ServerMessage::Message(msg, peers.lock().unwrap()[&ID].nickname.clone())).unwrap().into();
                                    for (_, peer) in peers.lock().unwrap().iter_mut().filter(|(&k, v)| k != ID) {
                                        peer.writer.start_send(serialized_msg.clone());
                                    }
                                    Ok(())
                                },
                                ClientMessage::Nickname(name) => {
                                    let old_nickname = peers.lock().unwrap()[&ID].nickname.clone();
                                    let name_clone = name.clone();
                                    peers.lock().unwrap().entry(ID).and_modify(|peer|{
                                        peer.nickname = name_clone;
                                    });
                                    let serialized_server_text: Bytes = serialize(&ServerMessage::ServerText(format!("{} changed their name to {}.", old_nickname, name))).unwrap().into();
                                    for peer in peers.lock().unwrap().values_mut() {
                                        peer.writer.start_send(serialized_server_text.clone());
                                    }
                                    Ok(())
                                },
                                ClientMessage::Quit => {
                                    let peer = peers.lock().unwrap().remove(&ID).unwrap();
                                    let serialized_quit: Bytes = serialize(&ServerMessage::Quit(peer.nickname)).unwrap().into();
                                    for peer in peers.lock().unwrap().values_mut() {
                                        peer.writer.start_send(serialized_quit.clone());
                                    }
                                    Ok(())
                                    // Err(Error::from(ErrorKind::ConnectionAborted)) // TODO: ensure an error here closes the connection to the client.
                                },
                                ClientMessage::Join(name) => {
                                    Ok(()) // Ignore this misbehaving message.
                                },
                            },
                            Err(err) => {
                                let peer = peers.lock().unwrap().remove(&ID).unwrap(); // TODO: ensure an error here closes the connection to the misbehaving client.
                                // TODO: send a notification that the client left.
                                let serialized_quit: Bytes = serialize(&ServerMessage::Quit(peer.nickname)).unwrap().into();
                                for peer in peers.lock().unwrap().values_mut() {
                                    peer.writer.start_send(serialized_quit.clone());
                                }
                                Ok(())
                                // Err(Error::from(ErrorKind::InvalidData))
                            },
                        };
                        for peer in peers.lock().unwrap().values_mut() {
                            peer.writer.poll_complete();
                        }
                        ret
                    });
                    let peers = peers_clone_three;
                    let h = h.and_then(move|()|{
                        let peer = peers.lock().unwrap().remove(&ID).unwrap();
                        let serialized_quit: Bytes = serialize(&ServerMessage::Quit(peer.nickname)).unwrap().into();
                        for peer in peers.lock().unwrap().values_mut() {
                            peer.writer.start_send(serialized_quit.clone());
                        }
                        for peer in peers.lock().unwrap().values_mut() {
                            peer.writer.poll_complete();
                        }
                        Ok(())
                    });
                    let peers = peers_clone_four;
                    let h = h.or_else(move|err|{
                        panic!("error: {}", err);
                        Ok(())
                    });
                    tokio::spawn(h);
                    Ok(())
                }).map_err(|err| panic!("annother server error: {}", err));
            tokio::spawn(frame_handler);
            Ok(())
        })
        .map_err(|err| {
            panic!("ERROR accepting new socket connection: {}", err);
        });
    tokio::run(server);
}
