use serde::{Deserialize, Serialize};

/// Client → Server 메시지
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Connect { uuid: String, nickname: String },
    Join { room: String },
    Leave,
    Message { text: String },
    List,
}

/// Server → Client 메시지
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Connected { uuid: String, nickname: String },
    Joined { room: String, members: Vec<MemberInfo> },
    Left { room: String },
    Message { from: String, room: String, text: String },
    UserJoined { display: String, room: String },
    UserLeft { display: String, room: String },
    RoomList { rooms: Vec<String> },
    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemberInfo {
    pub uuid: String,
    pub nickname: String,
}
