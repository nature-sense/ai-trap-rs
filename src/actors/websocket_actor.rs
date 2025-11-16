
//use std::sync::{Arc, Mutex, RwLock};
use futures_util::{select, FutureExt, SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use log::{debug, error, info};
use tokio_tungstenite::tungstenite::{Message as WsMessage};
use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio::net::{TcpListener, TcpStream};

use crate::framework::streams::BroadcastStream;

use async_broadcast::{ Receiver as BroadcastReceiver, Sender as BroadcastSender };
use prost::Message;
use crate::framework::actor::Actor;
use crate::messages::protobuf_msg::ProtobufMsg;
use crate::messages::raw_message::RawMessage;

type WsRxStream = SplitStream<WebSocketStream<TcpStream>>;
type WsTxStream = SplitSink<WebSocketStream<TcpStream>, WsMessage>;


pub struct WebsocketActor {
    protobuf_pub_rx :  BroadcastReceiver<ProtobufMsg>,
    protobuf_subs_tx : BroadcastSender<ProtobufMsg>,
}

impl WebsocketActor {
    pub(crate) fn new(
        protobuf_pub: BroadcastStream<ProtobufMsg>,
        protobuf_subs: BroadcastStream<ProtobufMsg>,
    ) -> Self {
        Self {
            protobuf_pub_rx: protobuf_pub.broadcast_receiver(),
            protobuf_subs_tx: protobuf_subs.broadcast_sender(),
        }
    }
}

impl Actor for WebsocketActor {
    async fn on_started(mut self) {

        debug!("Websocket actor started");

        //let mut tx_stream :  Mutex<Option<WsTxStream>> = Mutex::new(None);
        let (stream_tx, stream_rx) = async_channel::bounded::<Option<WsTxStream>>(4);

        // Task to listen on protobuf publish stream and send messages to the websocket
        // if it exists
        let _ = tokio::spawn(async move {
            let mut write_stream: Option<WsTxStream> = None;
            loop {
                select! {
                    prot_res = self.protobuf_pub_rx.recv().fuse() => {
                        match prot_res {
                            Ok(msg) => {
                                match write_stream {
                                    Some(ref mut stream) => {
                                        let raw_msg = msg.to_raw_message().unwrap();
                                        let ws_msg = WsMessage::Binary(raw_msg.message.into());
                                        let _ = stream.send(ws_msg).await;
                                    }
                                    None => {}
                                }
                            }
                            Err(_e) => {}
                        }
                    }
                    stream_res = stream_rx.recv().fuse() => {
                        match stream_res {
                            Ok(msg) => {
                                write_stream = msg;
                            }
                            Err(_e) => {}
                        }
                    }
                }
            }
        });

        let listener = TcpListener::bind("0.0.0.0:8096").await.unwrap();
        info!("Listening on 0.0.0.0:8096");

        loop {
            let (stream, peer_addr) = listener.accept().await.expect("Accept connection failed");
            let wss = accept_async(stream).await.expect("Failed to accept WebSocket connection");
            debug!("Connection request accepted");
            let (write, mut read) = wss.split();

            stream_tx.send(Some(write)).await.unwrap();
            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        if msg.is_binary() {
                            let raw_msg = RawMessage { message: msg.into_data().encode_to_vec() };
                            let msg = ProtobufMsg::from_raw_message(raw_msg).unwrap();
                            let _ = self.protobuf_subs_tx.broadcast(msg).await.unwrap();
                        } else if msg.is_close() {
                            debug!("Client {} sent a close message.", peer_addr);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message from {}: {}", peer_addr, e);
                        break;
                    }
                }
            }
            debug!("Connection closed");
            stream_tx.send(None).await.unwrap();
        };
    }
}
