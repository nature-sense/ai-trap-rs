use prost::Message as PbMessage;
use anyhow::{Context as ErrContext, Result};
use tungstenite::Bytes;

use crate::generated::protocol::ProtobufMessage;
use crate::messages::raw_message::RawMessage;

#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub struct ProtobufMsg {
    pub(crate) identifier: String,
    pub(crate) payload: Vec<u8>,
}

impl ProtobufMsg {
    pub(crate) fn to_raw_message(&self) -> Result<RawMessage> {
        let pm = ProtobufMessage {
            identifier: self.identifier.clone(),
            protobuf : self.payload.clone(),
        };
        let mut buf = Vec::new();
        pm.encode(&mut buf).context("Failed to encode protobuf message")?;
        Ok(RawMessage {message: buf})
    }

    pub fn from_raw_message(raw: RawMessage) -> Result<Self> {
        let pm = ProtobufMessage::decode(&raw.message[..])
            .context("failed to decode protobuf message")?; // Deserializes from the buffer
        let msg = ProtobufMsg {
            identifier: pm.identifier.trim().to_string(),
            payload: pm.protobuf,
        };
        Ok(msg)
    }

    pub fn _from_bytes(bytes: Bytes) -> Result<Self> {
        let pm = ProtobufMessage::decode(bytes)
            .context("failed to decode protobuf message")?; // Deserializes from the buffer
        let msg = ProtobufMsg {
            identifier: pm.identifier,
            payload: pm.protobuf,
        };
        Ok(msg)
    }
}


