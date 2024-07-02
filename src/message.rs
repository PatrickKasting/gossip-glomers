use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::node;

pub type Id = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "src")]
    pub source: node::Id,

    #[serde(rename = "dest")]
    pub destination: node::Id,

    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Body {
    Request {
        #[serde(flatten)]
        request: Request,

        #[serde(rename = "msg_id")]
        id: Id,
    },
    Response {
        #[serde(flatten)]
        response: Response,

        #[serde(rename = "msg_id")]
        id: Id,

        #[serde(rename = "in_reply_to")]
        request_id: Id,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Request {
    Init {
        node_id: node::Id,
        node_ids: Vec<node::Id>,
    },
    Echo {
        echo: String,
    },
    Generate,
    Broadcast {
        message: node::BroadcastMessage,
    },
    Read,
    Topology {
        topology: HashMap<node::Id, Vec<node::Id>>,
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
    Generate { id: node::Guid },

    #[serde(rename = "broadcast_ok")]
    Broadcast,

    #[serde(rename = "read_ok")]
    Read {
        messages: HashSet<node::BroadcastMessage>,
    },

    #[serde(rename = "topology_ok")]
    Topology,
}
