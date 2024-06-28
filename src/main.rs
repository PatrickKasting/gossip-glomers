use std::io::{stdin, BufRead};

use anyhow::{anyhow, Context, Ok};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    #[serde(rename = "src")]
    source: String,

    #[serde(rename = "dest")]
    destination: String,

    body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body {
    #[serde(rename = "msg_id")]
    message_id: Option<usize>,

    #[serde(rename = "in_reply_to")]
    request_id: Option<usize>,

    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
    Echo {
        echo: String,
    },
    EchoOk {
        echo: String,
    },
}

fn main() -> anyhow::Result<()> {
    let mut message_count = 0;

    let mut inputs = stdin().lock().lines();

    let initial_message = inputs
        .next()
        .context("initial message should be readable")??;
    let Message {
        source,
        destination,
        body: Body {
            message_id,
            request_id,
            payload,
        },
    } = serde_json::from_str(&initial_message)?;
    assert_eq!(request_id, None);
    let Payload::Init { node_id, node_ids } = payload else {
        return Err(anyhow!("initial message should have the type 'Init'"));
    };

    let response = Message {
        source: destination,
        destination: source,
        body: Body {
            message_id: Some(message_count),
            request_id: message_id,
            payload: Payload::InitOk,
        },
    };
    println!("{}", serde_json::to_string(&response)?);
    message_count += 1;

    for message in inputs {
        let message = message?;
        let Message {
            source,
            destination,
            body:
                Body {
                    message_id,
                    request_id,
                    payload,
                },
        } = serde_json::from_str(&message)?;
        assert_eq!(request_id, None);
        let Payload::Echo { echo } = payload else {
            return Err(anyhow!("initial message should have the type 'Init'"));
        };

        let response = Message {
            source: destination,
            destination: source,
            body: Body {
                message_id: Some(message_count),
                request_id: message_id,
                payload: Payload::EchoOk { echo },
            },
        };
        println!("{}", serde_json::to_string(&response)?);
        message_count += 1;
    }

    anyhow::Ok(())
}
