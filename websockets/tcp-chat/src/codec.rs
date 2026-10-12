#![allow(dead_code)]
use std::io;

use byteorder::{BigEndian, ByteOrder};
use ntex::codec::{Decoder, Encoder};
use ntex::util::{BufMut, BytePages, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json as json;

/// Client request
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd", content = "data")]
pub enum ChatRequest {
    /// List rooms
    List,
    /// Set name
    Name(String),
    /// Join rooms
    Join(String),
    /// Send message
    Message(String),
    /// Ping
    Ping,
}

/// Server response
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd", content = "data")]
pub enum ChatResponse {
    Ping,

    /// List of rooms
    Rooms(Vec<String>),

    /// Joined
    Joined(String),

    /// Message
    Message(String),
}

/// Codec for Client -> Server transport
pub struct ChatCodec;

impl Decoder for ChatCodec {
    type Item = ChatRequest;
    type Error = io::Error;

    fn decode(&self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            let _ = src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<ChatRequest>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ChatCodec {
    type Item = ChatResponse;
    type Error = io::Error;

    fn encode(&self, msg: ChatResponse, dst: &mut BytePages) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.put_u16(msg_ref.len() as u16);
        dst.extend_from_slice(msg_ref);

        Ok(())
    }
}

/// Codec for Server -> Client transport
pub struct ClientChatCodec;

impl Decoder for ClientChatCodec {
    type Item = ChatResponse;
    type Error = io::Error;

    fn decode(&self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            let _ = src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<ChatResponse>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ClientChatCodec {
    type Item = ChatRequest;
    type Error = io::Error;

    fn encode(&self, msg: ChatRequest, dst: &mut BytePages) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.put_u16(msg_ref.len() as u16);
        dst.extend_from_slice(msg_ref);

        Ok(())
    }
}
