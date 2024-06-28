use std::io;

use anyhow::Ok;

use crate::message::{Body, Message, Request, Response};

#[derive(Debug, Clone)]
pub struct Node {
    id: String,
    all_ids: Vec<String>,
    message_count: usize,
    next_guid: usize,
}

impl Node {
    pub fn new(writer: &mut impl io::Write, message: Message<Request>) -> anyhow::Result<Self> {
        assert!(
            matches!(message.body.payload, Request::Init { .. }),
            "first message should have type 'init'"
        );

        let mut node = Node {
            id: String::new(),
            all_ids: vec![],
            message_count: 0,
            next_guid: usize::MAX,
        };
        node.respond(writer, message)?;
        Ok(node)
    }

    pub fn respond(
        &mut self,
        writer: &mut impl io::Write,
        request: Message<Request>,
    ) -> anyhow::Result<()> {
        let response = self.response_message(request);
        serde_json::to_writer(&mut *writer, &response)?;
        writer.write(b"\n")?;
        writer.flush()?;
        self.message_count += 1;
        Ok(())
    }

    fn response_message(&mut self, request_message: Message<Request>) -> Message<Response> {
        let Message {
            source,
            destination,
            body,
        } = request_message;
        Message {
            source: destination,
            destination: source,
            body: self.response_body(body),
        }
    }

    fn response_body(&mut self, request_body: Body<Request>) -> Body<Response> {
        let Body {
            message_id,
            payload: request,
            ..
        } = request_body;
        Body {
            message_id: Some(self.message_count),
            request_id: message_id,
            payload: self.response(request),
        }
    }

    fn response(&mut self, request: Request) -> Response {
        match request {
            Request::Init { node_id, node_ids } => {
                self.id = node_id;
                self.all_ids = node_ids;
                self.all_ids.sort_unstable();
                self.next_guid = self.all_ids.iter().position(|id| id == &self.id).unwrap();
                Response::InitOk
            }
            Request::Echo { echo } => Response::EchoOk { echo },
            Request::Generate => {
                let guid = self.next_guid;
                self.next_guid += self.all_ids.len();
                Response::GenerateOk { id: guid }
            }
        }
    }
}
