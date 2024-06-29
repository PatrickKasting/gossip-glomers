use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "src")]
    pub source: String,

    #[serde(rename = "dest")]
    pub destination: String,

    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    #[serde(rename = "msg_id")]
    pub message_id: Option<usize>,

    #[serde(rename = "in_reply_to")]
    pub request_id: Option<usize>,

    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    Echo {
        echo: String,
    },
    Generate,
    Broadcast {
        message: usize,
    },
    Read,
    Topology {
        topology: HashMap<String, Vec<String>>,
    },

    InitOk,
    EchoOk {
        echo: String,
    },
    GenerateOk {
        id: usize,
    },
    BroadcastOk,
    ReadOk {
        messages: HashSet<usize>,
    },
    TopologyOk,
}
