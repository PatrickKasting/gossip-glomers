use std::io;

use anyhow::{Context, Ok, Result};

use crate::message::{Body, Message, Request, Response};

#[derive(Debug, Clone)]
pub struct Node {
    id: String,
    all_ids: Vec<String>,
    message_count: usize,
    next_guid: usize,
    broadcast_messages: Vec<usize>,
}

impl Node {
    pub fn new(writer: &mut impl io::Write, message: Message<Request>) -> Result<Self> {
        assert!(
            matches!(message.body.payload, Request::Init { .. }),
            "first message should have type 'init'"
        );

        let mut node = Node {
            id: String::new(),
            all_ids: vec![],
            message_count: 0,
            next_guid: usize::MAX,
            broadcast_messages: vec![],
        };
        node.respond(writer, message)?;
        Ok(node)
    }

    pub fn respond(
        &mut self,
        writer: &mut impl io::Write,
        request: Message<Request>,
    ) -> Result<()> {
        let response = self.response_message(request)?;
        serde_json::to_writer(&mut *writer, &response)?;
        writer.write(b"\n")?;
        writer.flush()?;
        self.message_count += 1;
        Ok(())
    }

    fn response_message(&mut self, request_message: Message<Request>) -> Result<Message<Response>> {
        let Message {
            source,
            destination,
            body,
        } = request_message;
        Ok(Message {
            source: destination,
            destination: source,
            body: self.response_body(body)?,
        })
    }

    fn response_body(&mut self, request_body: Body<Request>) -> Result<Body<Response>> {
        let Body {
            message_id,
            payload: request,
            ..
        } = request_body;
        Ok(Body {
            message_id: Some(self.message_count),
            request_id: message_id,
            payload: self.response(request)?,
        })
    }

    fn response(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::Init {
                node_id,
                mut node_ids,
            } => {
                node_ids.sort_unstable();
                let index = node_ids
                    .iter()
                    .position(|id| id == &node_id)
                    .context("node id should be in the list of all ids")?;

                self.id = node_id;
                self.all_ids = node_ids;
                self.next_guid = index;
                Ok(Response::InitOk)
            }
            Request::Echo { echo } => Ok(Response::EchoOk { echo }),
            Request::Generate => {
                self.next_guid += self.all_ids.len();
                Ok(Response::GenerateOk { id: self.next_guid })
            }
            Request::Broadcast { message } => {
                self.broadcast_messages.push(message);
                Ok(Response::BroadcastOk)
            }
            Request::Read => Ok(Response::ReadOk {
                messages: self.broadcast_messages.clone(),
            }),
            Request::Topology { topology: _ } => Ok(Response::TopologyOk),
        }
    }
}
