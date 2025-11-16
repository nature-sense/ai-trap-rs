use anyhow::Result;
use chrono::{DateTime, Local};
use log::{debug, warn};
use native_db::*;
use native_model::{native_model, Model};
use prost::Message as PbMessage;
use serde::{Deserialize, Serialize};

use crate::generated::sessions::{Detection, Session, SessionDetails};
use crate::messages::protobuf_msg::ProtobufMsg;

use crate::framework::actor::Actor;
use crate::framework::streams::BroadcastStream;
use async_broadcast::{Receiver as BroadcastReceiver, Sender as BroadcastSender};
use once_cell::sync::Lazy;
//use futures_util::StreamExt;

// ==============================================================================
// Database models
// ==============================================================================
#[derive(Serialize, Deserialize, Debug)]
#[native_model(id = 1, version = 1)]
#[native_db]
struct DetectionModel {
    #[primary_key]
    detection: i32,
    #[secondary_key]
    session: String,
    created: i64,
    updated: i64,
    score: f32,
    clazz: i32,
    width: i32,
    height: i32,
    image: Vec<u8>,
}

impl DetectionModel {
    fn to_event(self) -> ProtobufMsg {
        ProtobufMsg {
            identifier: "detection".to_string(),
            payload: Detection {
                session: self.session,
                detection: self.detection,
                created: self.created,
                updated: self.updated,
                score: self.score,
                clazz: self.clazz,
                width: self.width,
                height: self.height,
                image: Some(self.image),
            }
            .encode_to_vec(),
        }
    }
}

#[native_model(id = 2, version = 1)]
#[native_db]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SessionModel {
    #[primary_key]
    session: String,
    #[secondary_key]
    active: i32,
    opened: i64,
    closed: Option<i64>,
}

impl SessionModel {
    fn to_event(self, event: &str, detections: i32) -> ProtobufMsg {
        ProtobufMsg {
            identifier: event.to_string(),
            payload: SessionDetails {
                session: self.session,
                active: self.active == 1,
                opened: self.opened,
                closed: self.closed,
                detections,
            }
            .encode_to_vec(),
        }
    }
}
// ==============================================================================
// Database
// ==============================================================================

static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<SessionModel>().unwrap();
    models.define::<DetectionModel>().unwrap();
    models
});

pub struct SessionsActor {
    protobuf_pub_tx: BroadcastSender<ProtobufMsg>,
    protobuf_subs_rx: BroadcastReceiver<ProtobufMsg>,
    db: Database<'static>,
}

impl SessionsActor {
    pub(crate) fn new(
        protobuf_pub: BroadcastStream<ProtobufMsg>,
        protobuf_subs: BroadcastStream<ProtobufMsg>,
        db_location: String,
    ) -> Self {
        Self {
            protobuf_pub_tx: protobuf_pub.broadcast_sender(),
            protobuf_subs_rx: protobuf_subs.broadcast_receiver(),
            db: Builder::new()
                .create(&MODELS, db_location)
                .expect("Failed to create database"),
        }
    }

    async fn open_session(&mut self) -> Result<()> {
        debug!("Opening session");
        let local_now: DateTime<Local> = Local::now();
        let session_id = local_now.format("%Y%m%d%H%M%S").to_string();
        let opened = local_now.timestamp_millis();

        let rw = self.db.rw_transaction()?;

        let mut close_event: Option<ProtobufMsg> = None;

        // Check to see if there is an active session
        // If so, set it to inactive and send close event
        for active in rw.scan().secondary(SessionModelKey::active)?.range(1..1)? {
            let orig: SessionModel = active?;
            let mut new = orig.clone();
            new.active = 0;
            new.closed = Option::from(opened);

            let detections = rw
                .scan()
                .secondary::<DetectionModel>(DetectionModelKey::session)?
                .start_with(orig.session.clone())
                .iter()
                .count() as i32;
            close_event = Some(new.clone().to_event("session.closed", detections));
            rw.update(orig.clone(), new.clone())?;
        }

        let session = SessionModel {
            session: session_id,
            active: 1,
            opened,
            closed: None,
        };
        rw.insert(session.clone())?;
        rw.commit()?;

        // send the events after committing
        if close_event.is_some() {
            let msg = close_event.unwrap();
            self.protobuf_pub_tx.broadcast(msg).await?;
        }

        debug!("Session opened");
        let add_event = session.to_event("session.opened", 0);
        self.protobuf_pub_tx.broadcast(add_event).await?;

        Ok(())
    }

    async fn all_sessions(&mut self) -> Result<()> {
        debug!("Reading sessions from database");

        let r = self.db.r_transaction()?;

        // Get all values
        for res in r.scan().primary::<SessionModel>()?.all()? {
            let session = res?;
            let details_event = session.to_event("session.details", 0);
            self.protobuf_pub_tx.broadcast(details_event).await?;
        }
        debug!("Finished reading sessions from database");
        Ok(())
    }
}

impl Actor for SessionsActor {
    async fn on_started(mut self) {
        debug!("Sessions actor started");

        loop {
            let res = self.protobuf_subs_rx.recv_direct().await;
            match res {
                Ok(msg) => {
                    match msg.identifier.as_str() {
                        "session.open" => {
                            debug!("Open session received");
                            let result = self.open_session().await;
                            match result {
                                Ok(_) => {
                                    debug!("Finished adding session to database");
                                }
                                Err(e) => {
                                    warn!("Error adding session to database {}", e);
                                }
                            }
                        }

                        "session.close" => {}

                        "session.all" => {
                            debug!("Received sessions.all");
                            self.all_sessions().await.unwrap();
                        }

                        "session.detections" => {
                            debug!("Received session.detections");
                            let sess = Session::decode(&msg.payload[..]).unwrap();
                            //let r = state.lock().unwrap().db.r_transaction().unwrap();

                            // Get all values
                            for res in self
                                .db.r_transaction()
                                .unwrap()
                                .scan()
                                .secondary::<DetectionModel>(DetectionModelKey::session)
                                .unwrap()
                                .start_with(sess.session)
                                .unwrap() {

                                let model = res.unwrap();
                                let detection_event = model.to_event();
                                self.protobuf_pub_tx
                                    .broadcast(detection_event)
                                    .await
                                    .unwrap();
                            }
                        },
                        &_ => { }
                    }
                    //_ => {}
                }
                Err(_) => {}
            }
        }
    }
}
