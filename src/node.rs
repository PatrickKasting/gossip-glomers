use std::{collections::HashSet, io::Write};

use anyhow::{Context, Ok, Result};

use crate::message::{Body, Message, Payload};

#[derive(Debug, Clone)]
pub struct Node<Output: Write> {
    output: Output,
    id: String,
    all_node_ids: Vec<String>,
    next_message_id: usize,
    next_guid: usize,
    broadcast_messages: HashSet<usize>,
    neighbors: Vec<String>,
}

impl<Output: Write> Node<Output> {
    pub fn new(output: Output, init: Message) -> Result<Self> {
        assert!(
            matches!(init.body.payload, Payload::Init { .. }),
            "first message should have type 'init'"
        );

        let mut node = Node {
            output,
            id: String::new(),
            all_node_ids: vec![],
            next_message_id: 0,
            next_guid: usize::MAX,
            broadcast_messages: HashSet::new(),
            neighbors: vec![],
        };
        node.handle(init)?;
        Ok(node)
    }

    pub fn handle(&mut self, message: Message) -> Result<()> {
        let Message {
            source,
            destination,
            body:
                Body {
                    message_id,
                    payload,
                    ..
                },
        } = message;
        if let Some(payload) = self.response(payload)? {
            let response = Message {
                source: destination,
                destination: source,
                body: Body {
                    message_id: Some(self.next_message_id()),
                    request_id: message_id,
                    payload,
                },
            };
            self.send(&response)?;
        }
        Ok(())
    }

    fn response(&mut self, request: Payload) -> Result<Option<Payload>> {
        match request {
            Payload::Init {
                node_id,
                mut node_ids,
            } => {
                node_ids.sort_unstable();
                let index = node_ids
                    .iter()
                    .position(|id| id == &node_id)
                    .context("node id should be in the list of all ids")?;

                self.id = node_id;
                self.all_node_ids = node_ids.clone();
                self.next_guid = index;
                self.neighbors = node_ids;
                Ok(Some(Payload::InitOk))
            }
            Payload::Echo { echo } => Ok(Some(Payload::EchoOk { echo })),
            Payload::Generate => {
                self.next_guid += self.all_node_ids.len();
                Ok(Some(Payload::GenerateOk { id: self.next_guid }))
            }
            Payload::Broadcast { message } => {
                if self.broadcast_messages.insert(message) {
                    self.broadcast_to_neighbors(message)?;
                }
                Ok(Some(Payload::BroadcastOk))
            }
            Payload::Read => Ok(Some(Payload::ReadOk {
                messages: self.broadcast_messages.clone(),
            })),
            Payload::Topology { mut topology } => {
                self.neighbors = topology
                    .remove(&self.id)
                    .context("this node should appear in the topology")?;
                Ok(Some(Payload::TopologyOk))
            }
            Payload::InitOk
            | Payload::EchoOk { .. }
            | Payload::GenerateOk { .. }
            | Payload::BroadcastOk
            | Payload::ReadOk { .. }
            | Payload::TopologyOk => Ok(None),
        }
    }

    fn broadcast_to_neighbors(&mut self, message: usize) -> Result<()> {
        for neighbor in self.neighbors.clone() {
            let message = self.request(&neighbor, Payload::Broadcast { message });
            self.send(&message)?;
        }
        Ok(())
    }

    fn request(&mut self, destination: &str, payload: Payload) -> Message {
        Message {
            source: self.id.clone(),
            destination: destination.to_owned(),
            body: Body {
                message_id: Some(self.next_message_id()),
                request_id: None,
                payload,
            },
        }
    }

    fn send(&mut self, message: &Message) -> Result<()> {
        serde_json::to_writer(&mut self.output, message)?;
        self.output.write(b"\n")?;
        self.output.flush()?;
        Ok(())
    }

    fn next_message_id(&mut self) -> usize {
        self.next_message_id += 1;
        return self.next_message_id;
    }
}
