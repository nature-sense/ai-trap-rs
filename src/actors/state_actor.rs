use async_broadcast::{ Receiver as BroadcastReceiver, Sender as BroadcastSender };
//use async_channel::{Receiver as ChannelReceiver, Sender as ChannelSender};

use anyhow::{Context as ErrContext, Result};
use futures_util::StreamExt;
use log::debug;

use prost::Message;
use crate::framework::actor::Actor;
use crate::framework::streams::BroadcastStream;
use crate::generated::control::State;
use crate::messages::protobuf_msg::ProtobufMsg;

#[derive(Clone, Debug)]
pub struct StateActor {
    protobuf_pub_tx :  BroadcastSender<ProtobufMsg>,
    protobuf_subs_tx : BroadcastSender<ProtobufMsg>,
    protobuf_subs_rx : BroadcastReceiver<ProtobufMsg>,
    capture_state : bool,
    streaming_state : bool,
}

impl StateActor {
    pub(crate) fn new(
        protobuf_pub :  BroadcastStream<ProtobufMsg>,
        protobuf_subs : BroadcastStream<ProtobufMsg>,
    ) -> Self {
        Self {
            protobuf_pub_tx : protobuf_pub.broadcast_sender(),
            protobuf_subs_tx : protobuf_subs.broadcast_sender(),
            protobuf_subs_rx : protobuf_subs.broadcast_receiver(),
            capture_state: false,
            streaming_state: false,
        }
    }
    
    async fn encode_state(self, identifier : &str, state : bool) -> Result<ProtobufMsg, anyhow::Error> {
        Ok(ProtobufMsg {
            identifier: identifier.to_string(),
            payload: State { state }.encode_to_vec()
        })
    }

    fn decode_state(self, buf : Vec<u8>) -> Result<bool> {
        let state = State::decode(&buf[..])
            .with_context(|| "Failed to decode state")?;
        Ok(state.state)
    }
}

impl Actor for StateActor {
    async fn on_started(mut self) {
        debug!("State actor started");

        loop {
            let res = self.protobuf_subs_rx.next().await;
            match res {
                Some(msg) => {
                    debug!("ProtobufMsg received identifier = [{}]", msg.identifier);
                    match msg.identifier.as_str() {
                        "state.capture.get" => {
                            debug!("Capture state is {}", self.capture_state);
                            let state_msg = self
                                .clone()
                                .encode_state("state.capture", self.capture_state)
                                .await.unwrap();
                            let _ = self.protobuf_pub_tx.broadcast(state_msg).await;
                        }
                        "state.capture.set" => {
                            let new_state = self
                                .clone()
                                .decode_state(msg.payload)
                                .unwrap();
                            if new_state != self.capture_state {
                                self.capture_state = new_state;

                                if self.capture_state {
                                    let msg = self
                                        .clone()
                                        .encode_state("session.state.set", true)
                                        .await
                                        .unwrap();
                                    self.protobuf_subs_tx.broadcast(msg).await.expect("Failed to send");
                                    let msg = self
                                        .clone()
                                        .encode_state("camera.state.set", true)
                                        .await
                                        .unwrap();
                                    self.protobuf_subs_tx.broadcast(msg).await.expect("Failed to send");
                                }
                                debug!("Capture state set to {}", self.capture_state);
                                let state_msg = self
                                    .clone()
                                    .encode_state("state.capture", self.capture_state)
                                    .await
                                    .unwrap();
                                self.protobuf_pub_tx.broadcast(state_msg).await.unwrap();
                            }
                        }

                        "state.streaming.get" => {
                            debug!("Streaming state is {}",
                                self.streaming_state);
                            let state_msg = self
                                .clone()
                                .encode_state("state.streaming", self.streaming_state)
                                .await
                                .unwrap();
                            self.protobuf_pub_tx.broadcast(state_msg).await.unwrap();
                        }
                        "state.streaming.set" => {
                            let set_stream_fn = async || -> Result<()> {
                                let new_state = self.clone().decode_state(msg.payload)?;
                                self.clone().streaming_state = new_state;

                                debug!("Stream state set to {}", self.streaming_state);
                                let state_msg = self
                                    .clone()
                                    .encode_state("state.stream", self.streaming_state)
                                    .await?;
                                self.protobuf_pub_tx.broadcast(state_msg).await?;
                                Ok(())
                            };
                            let _ = set_stream_fn().await;
                        }
                        _ => { debug!("unhandled message {}", msg.identifier.as_str())}
                    }
                }
                None => {}
            }
        };
    }
}

