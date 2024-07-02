use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub type NodeId = String;
pub type MessageId = usize;
pub type BroadcastMessage = usize;
pub type Guid = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "src")]
    pub source: NodeId,

    #[serde(rename = "dest")]
    pub destination: NodeId,

    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Body {
    Request {
        #[serde(flatten)]
        request: Request,

        #[serde(rename = "msg_id")]
        id: MessageId,
    },
    Response {
        #[serde(flatten)]
        response: Response,

        #[serde(rename = "msg_id")]
        id: MessageId,

        #[serde(rename = "in_reply_to")]
        request_id: MessageId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Request {
    Init {
        node_id: NodeId,
        node_ids: Vec<NodeId>,
    },
    Echo {
        echo: String,
    },
    Generate,
    Broadcast {
        message: BroadcastMessage,
    },
    Read,
    Topology {
        topology: HashMap<NodeId, Vec<NodeId>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "init_ok")]
    Init,

    #[serde(rename = "echo_ok")]
    Echo { echo: String },

    #[serde(rename = "generate_ok")]
    Generate { id: Guid },

    #[serde(rename = "broadcast_ok")]
    Broadcast,

    #[serde(rename = "read_ok")]
    Read { messages: HashSet<BroadcastMessage> },

    #[serde(rename = "topology_ok")]
    Topology,
}
