use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// A header-safe representation; TLS is still required on untrusted networks.
// Never put credentials in room URLs, invitations, discovery results or logs.
pub fn authorization(password: &str) -> String {
    if password.is_empty() {
        String::new()
    } else {
        format!("Bearer {:x}", Sha256::digest(password.as_bytes()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEntry {
    #[serde(default)]
    pub code: String,
    pub name: String,
    pub players: usize,
    #[serde(default)]
    pub capacity: usize,
    #[serde(default)]
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub map: String,
    pub x: f32,
    pub y: f32,
    pub moving: bool,
    pub facing: bool,
    #[serde(default)]
    pub indoors: bool,
    #[serde(default)]
    pub fishing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Paint {
        map: String,
        x: u32,
        y: u32,
        tool: crate::pixels::Tool,
    },
    Interact {
        map: String,
        object: u32,
    },
    WorldReady {
        revision: String,
    },
    Move {
        map: String,
        x: f32,
        y: f32,
        moving: bool,
        facing: bool,
        #[serde(default)]
        indoors: bool,
        #[serde(default)]
        fishing: bool,
    },
    Chat {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    World {
        world: Box<crate::World>,
    },
    Welcome {
        you: u32,
        players: Vec<Player>,
        #[serde(default)]
        states: Vec<ObjectState>,
        pixels: Vec<crate::pixels::Update>,
    },
    Pixels {
        update: crate::pixels::Update,
    },
    ObjectChanged {
        value: ObjectState,
    },
    Joined {
        player: Player,
    },
    Moved {
        player: Player,
    },
    Chat {
        id: u32,
        text: String,
    },
    Left {
        id: u32,
    },
}

pub fn clean(text: &str, limit: usize) -> String {
    text.chars().filter(|c| !c.is_control() && !matches!(*c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}')).take(limit).collect::<String>().trim().to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectState {
    pub map: String,
    pub object: u32,
    pub state: String,
}
