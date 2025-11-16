use log::{debug, error};
use ort::session::Session;
use crate::framework::actor::Actor;
use crate::messages::camera_frame::CameraFrame;
use crate::messages::protobuf_msg::ProtobufMsg;


use crate::framework::streams::BroadcastStream;
use crate::framework::streams::ChannelStream;

use async_broadcast::{ Receiver as BroadcastReceiver, Sender as BroadcastSender };
use async_channel::Receiver as ChannelReceiver;

pub struct DetectionActor {
    frame_rx: ChannelReceiver<CameraFrame>,
    protobuf_pub_tx: BroadcastSender<ProtobufMsg>,
    protobuf_subs_rx: BroadcastReceiver<ProtobufMsg>,
}

impl DetectionActor {
    pub fn new(
        frame_receiver: ChannelStream<CameraFrame>,
        protobuf_pub: BroadcastStream<ProtobufMsg>,
        protobuf_subs: BroadcastStream<ProtobufMsg>
    ) -> Self {
        Self {
            frame_rx : frame_receiver.channel_receiver(),
            protobuf_pub_tx : protobuf_pub.broadcast_sender(),
            protobuf_subs_rx : protobuf_subs.broadcast_receiver(),
        }
    }
}

impl Actor for DetectionActor {
    async fn on_started(mut self) {
        debug!("Detection actor started");

        let result = Session::builder()
            .unwrap().
            commit_from_file("models/insects-320.onnx");
        
        match result {
            Ok(_model) => { debug!("Model loaded"); },
            Err(e) => { error!("Error {}", e)}
        }
        
        let res = self.protobuf_subs_rx.recv().await;
        match res {
            Ok(msg) => {
                debug!("->> ProtobufMsg {}", msg.identifier);
            }
            Err(_) => {
                debug!("No message received");
                
            }
        }
        loop {
            let res = self.frame_rx.recv().await;
            match res {
                Ok(_msg) => {
                    debug!("->> Frame ");
                }
                Err(_) => {
                    debug!("No message received");
                }
            }
        }
    }
}