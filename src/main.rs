mod actors;
mod generated;
mod messages;
mod framework;

use std::thread::JoinHandle;
use simplelog::*;

use crate::actors::camera_actor::CameraActor;
use crate::actors::detection_actor::DetectionActor;
use crate::actors::sessions_actor::SessionsActor;
use crate::actors::state_actor::StateActor;
use crate::actors::websocket_actor::WebsocketActor;
use crate::framework::actor::Actor;
use crate::framework::streams::{BroadcastStream, ChannelStream};
use crate::messages::camera_frame::CameraFrame;
use crate::messages::protobuf_msg::ProtobufMsg;

#[tokio::main]
async fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        ]
    ).unwrap();

    let protobuf_pub: BroadcastStream<ProtobufMsg> = BroadcastStream::new(10);
    let protobuf_subs: BroadcastStream<ProtobufMsg> = BroadcastStream::new(10);
    let camera_frame: ChannelStream<CameraFrame> = ChannelStream::new(10);

    let session_actor = SessionsActor::new(
        protobuf_pub.clone(),
        protobuf_subs.clone(),
        "location".to_string()
    );
    let state_actor = StateActor::new(
        protobuf_pub.clone(),
        protobuf_subs.clone()
    );
    let camera_actor = CameraActor::new(
        camera_frame.clone(),
        protobuf_pub.clone(),
        protobuf_subs.clone()
    );
    let detection_actor = DetectionActor::new(
        camera_frame.clone(),
        protobuf_pub.clone(),
        protobuf_subs.clone()
    );

    let websocket_actor = WebsocketActor::new(
        protobuf_pub.clone(),
        protobuf_subs.clone(),
    );

    let threads : Vec<JoinHandle<()>> = vec![
        session_actor.start().await,
        state_actor.start().await,
        camera_actor.start().await,
        detection_actor.start().await,
        websocket_actor.start().await
    ];

    for thread in threads {
        thread.join().unwrap();
    }
}

