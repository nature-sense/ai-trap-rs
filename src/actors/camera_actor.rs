use anyhow::Context as Ctx;

use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, CameraFormat, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{query, CallbackCamera};

//use crate::generated::sessions::Session;
use crate::messages::camera_frame::CameraFrame;
use crate::messages::protobuf_msg::ProtobufMsg;

use crate::framework::streams::BroadcastStream;
use crate::framework::streams::ChannelStream;

use async_broadcast::{ Receiver as BroadcastReceiver, Sender as BroadcastSender };
use async_channel::Sender as ChannelSender;

use log::debug;
use prost::Message as Msg;
use crate::framework::actor::Actor;
use crate::generated::control::State;

pub struct _StartCamera;

pub struct CameraActor {
    frame_tx: ChannelSender<CameraFrame>,
    protobuf_pub_tx: BroadcastSender<ProtobufMsg>,
    protobuf_subs_rx: BroadcastReceiver<ProtobufMsg>,
    camera: Option<CallbackCamera>,
    format: Option<CameraFormat>,
}

impl CameraActor {
    pub(crate) fn new(
        frame_sender: ChannelStream<CameraFrame>,
        protobuf_pub: BroadcastStream<ProtobufMsg>,
        protobuf_subs: BroadcastStream<ProtobufMsg>,
    ) -> Self {
        Self {
            frame_tx : frame_sender.channel_sender(),
            protobuf_pub_tx : protobuf_pub.broadcast_sender(),
            protobuf_subs_rx : protobuf_subs.broadcast_receiver(),
            camera: None,
            format: None,
        }
    }

    fn decode_state(self, buf : Vec<u8>) -> anyhow::Result<bool> {
        let state = State::decode(&buf[..])
            .with_context(|| "Failed to decode state")?;
        Ok(state.state)
    }
}
impl Actor for CameraActor {

    async fn on_started(mut self) {
        debug!("Camera actor started");

        let _ = query(ApiBackend::Auto).unwrap();
        //cameras.iter().for_each(|cam| println!("{:?}", cam));
        //let index = CameraIndex::Index(0);
        //let camera = cameras.first().unwrap();
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let camera = CallbackCamera::new(CameraIndex::Index(0), format, |_| {}).unwrap();

        let _known = camera.camera_controls_known_camera_controls().unwrap();
        let _controls = camera.camera_controls().unwrap();
        //controls.iter().for_each(|ctl |  println!("{:?}", ctl));
        let cam_format = camera.camera_format().unwrap();
        debug!("{:?}", cam_format);

        loop {
            let res = self.protobuf_subs_rx.recv().await;
            match res {
                Ok(msg) => {
                    debug!("->> ProtobufMsg {}", msg.identifier);
                    match msg.identifier.as_str() {
                        "camera.get" => {}
                        "camera.state.set" => match self.camera.take() {
                            Some(mut camera) => {
                                tokio::spawn(async move {
                                    loop {
                                        let res = camera.poll_frame();
                                        match res {
                                            Ok(_buffer) => {
                                                debug!("Frame")
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                });
                            }
                            None => {}
                        },
                        _ => {}
                    }
                }
                Err(_) => {
                    debug!("No message received");
                }
            }
        }
    }
}
