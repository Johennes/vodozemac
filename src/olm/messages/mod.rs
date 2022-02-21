// Copyright 2021 Damir Jelić
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod message;
mod pre_key;

pub use message::Message;
pub use pre_key::PreKeyMessage;
use serde::{Deserialize, Serialize};

use crate::DecodeError;

#[derive(Debug, Clone, PartialEq)]
pub enum OlmMessage {
    Normal(Message),
    PreKey(PreKeyMessage),
}

impl From<Message> for OlmMessage {
    fn from(m: Message) -> Self {
        Self::Normal(m)
    }
}

impl From<PreKeyMessage> for OlmMessage {
    fn from(m: PreKeyMessage) -> Self {
        Self::PreKey(m)
    }
}

#[derive(Serialize, Deserialize)]
struct MessageSerdeHelper {
    #[serde(rename = "type")]
    message_type: usize,
    #[serde(rename = "body")]
    ciphertext: String,
}

impl Serialize for OlmMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (message_type, ciphertext) = self.clone().to_parts();

        let message = MessageSerdeHelper { message_type, ciphertext };

        message.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OlmMessage {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = MessageSerdeHelper::deserialize(d)?;

        OlmMessage::from_parts(value.message_type, &value.ciphertext)
            .map_err(serde::de::Error::custom)
    }
}

impl OlmMessage {
    pub fn from_parts(message_type: usize, ciphertext: &str) -> Result<Self, DecodeError> {
        match message_type {
            0 => Ok(Self::PreKey(PreKeyMessage::try_from(ciphertext)?)),
            1 => Ok(Self::Normal(Message::try_from(ciphertext)?)),
            m => Err(DecodeError::MessageType(m)),
        }
    }

    pub fn ciphertext(&self) -> &[u8] {
        match self {
            OlmMessage::Normal(m) => &m.ciphertext,
            OlmMessage::PreKey(m) => &m.message.ciphertext,
        }
    }

    pub fn message_type(&self) -> MessageType {
        match self {
            OlmMessage::Normal(_) => MessageType::Normal,
            OlmMessage::PreKey(_) => MessageType::PreKey,
        }
    }

    pub fn to_parts(self) -> (usize, String) {
        let message_type = self.message_type();

        match self {
            OlmMessage::Normal(m) => (message_type.into(), m.to_base64()),
            OlmMessage::PreKey(m) => (message_type.into(), m.to_base64()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum MessageType {
    PreKey = 0,
    Normal = 1,
}

impl TryFrom<usize> for MessageType {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageType::PreKey),
            1 => Ok(MessageType::Normal),
            _ => Err(()),
        }
    }
}

impl From<MessageType> for usize {
    fn from(value: MessageType) -> usize {
        value as usize
    }
}

#[cfg(test)]
use olm_rs::session::OlmMessage as LibolmMessage;

#[cfg(test)]
impl From<LibolmMessage> for OlmMessage {
    fn from(other: LibolmMessage) -> Self {
        let (message_type, ciphertext) = other.to_tuple();

        Self::from_parts(message_type.into(), &ciphertext).expect("Can't decode a libolm message")
    }
}

#[cfg(test)]
impl From<OlmMessage> for LibolmMessage {
    fn from(value: OlmMessage) -> LibolmMessage {
        match value {
            OlmMessage::Normal(m) => LibolmMessage::from_type_and_ciphertext(1, m.to_base64())
                .expect("Can't create a valid libolm message"),
            OlmMessage::PreKey(m) => LibolmMessage::from_type_and_ciphertext(0, m.to_base64())
                .expect("Can't create a valid libolm pre-key message"),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::*;

    #[test]
    fn message_type_from_usize() {
        assert_eq!(
            MessageType::try_from(0),
            Ok(MessageType::PreKey),
            "0 should denote a pre-key Olm message"
        );
        assert_eq!(
            MessageType::try_from(1),
            Ok(MessageType::Normal),
            "1 should denote a normal Olm message"
        );
        assert!(
            MessageType::try_from(2).is_err(),
            "2 should be recognized as an unknown Olm message type"
        );
    }

    #[test]
    fn olm_message_from_json() -> Result<()> {
        let value = json!({
            "type": 0,
            "body": "AwoghAEuxPZ+w7M3pgUae4tDNiggUpOsQ/zci457VAtiAEYSIO3xOKRDBW\
                     KicIfxjSmYCYZ9DD4RMLjvvclbMlE5yIEWGiApLrCr853CKlPpW4Bi7S8y\
                     kRcejJ0lq7AfYLXKCjKdHSJPAwoghw3+P+cajhWj9Qzp5g87h+tbpiuh5w\
                     EaeUppqmWqug4QASIgRhZ2cgZcIWQbIa23R7U4y1Mo1R/tLCaMU+xjzRV5\
                     smGsCrJ6AHwktg",
        });

        let message: OlmMessage = serde_json::from_value(value.clone())?;

        let serialized = serde_json::to_value(message)?;

        assert_eq!(value, serialized, "The serialization cycle isn't a noop");

        Ok(())
    }
}
