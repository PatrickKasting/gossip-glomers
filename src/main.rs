mod message;

use std::{
    collections::HashSet,
    io::{self, stdout, BufRead, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
    thread,
};

use anyhow::Context;
use message::{Body, BroadcastMessage, Message, MessageId, NodeId, Request, Response};
use once_cell::sync::Lazy;

static NODE_ID: OnceLock<NodeId> = OnceLock::new();
static ALL_NODE_IDS: OnceLock<Vec<NodeId>> = OnceLock::new();
static NEXT_MESSAGE_ID: AtomicUsize = AtomicUsize::new(0);

static NEXT_GUID: AtomicUsize = AtomicUsize::new(usize::MAX);

static BROADCAST_MESSAGES_RECEIVED: Lazy<Mutex<HashSet<BroadcastMessage>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

static NEIGHBORS: OnceLock<Vec<NodeId>> = OnceLock::new();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (stdin_sender, stdin_receiver) = tokio::sync::mpsc::unbounded_channel();
    let stdin_thread = spawn_stdin_thread(stdin_sender)?;

    main_loop(stdin_receiver).await;

    stdin_thread
        .join()
        .expect("thread should be finished because the receiver is closed")?;
    anyhow::Ok(())
}

async fn main_loop(mut stdin_receiver: tokio::sync::mpsc::UnboundedReceiver<String>) {
    while let Some(message) = stdin_receiver.recv().await {
        tokio::spawn(async move {
            let message = serde_json::from_str(&message)?;
            handle_message(message)
        });
    }
}

fn handle_message(message: Message) -> anyhow::Result<()> {
    let Message { source, body, .. } = message;
    match body {
        Body::Request { id, request } => handle_request(source, id, request),
        Body::Response {
            id,
            request_id,
            response,
        } => handle_response(source, id, request_id, response),
    }
}

fn handle_request(source: NodeId, request_id: MessageId, request: Request) -> anyhow::Result<()> {
    let response_body = Body::Response {
        id: next_message_id(),
        request_id,
        response: response(request, &source)?,
    };
    let response_message = Message {
        source: node_id()?.clone(),
        destination: source,
        body: response_body,
    };
    send(&response_message)?;
    anyhow::Ok(())
}

fn response(request: Request, source: &NodeId) -> anyhow::Result<Response> {
    match request {
        Request::Init {
            node_id,
            mut node_ids,
        } => {
            node_ids.sort_unstable();
            let index = node_ids
                .iter()
                .position(|other| other == &node_id)
                .context("node id should be in the list of ids")?;

            NODE_ID.get_or_init(|| node_id);
            ALL_NODE_IDS.get_or_init(|| node_ids);
            NEXT_GUID.store(index, Ordering::Relaxed);

            anyhow::Ok(Response::InitOk)
        }
        Request::Echo { echo } => anyhow::Ok(Response::EchoOk { echo }),
        Request::Generate => {
            let number_of_nodes = all_node_ids()?.len();
            let guid = NEXT_GUID.fetch_add(number_of_nodes, Ordering::Relaxed);
            anyhow::Ok(Response::GenerateOk { id: guid })
        }
        Request::Broadcast { message } => {
            if broadcast_messages_received().insert(message) {
                let destinations = neighbors()?
                    .iter()
                    .filter(|&neighbor| neighbor != source)
                    .cloned();
                send_requests(Request::Broadcast { message }, destinations)?;
            };
            anyhow::Ok(Response::BroadcastOk)
        }
        Request::Read => anyhow::Ok(Response::ReadOk {
            messages: broadcast_messages_received().clone(),
        }),
        Request::Topology { mut topology } => {
            let neighbors = topology
                .remove(node_id()?)
                .context("node id should appear as key in received topology")?;
            NEIGHBORS.get_or_init(|| neighbors);
            anyhow::Ok(Response::TopologyOk)
        }
    }
}

fn handle_response(
    source: NodeId,
    response_id: MessageId,
    request_id: MessageId,
    response: Response,
) -> anyhow::Result<()> {
    todo!()
}

fn send_requests(
    request: Request,
    destinations: impl IntoIterator<Item = NodeId>,
) -> anyhow::Result<()> {
    for destination in destinations {
        send_request(request.clone(), destination)?;
    }
    anyhow::Ok(())
}

fn send_request(request: Request, destination: NodeId) -> anyhow::Result<()> {
    let body = Body::Request {
        request: request,
        id: next_message_id(),
    };
    let message = Message {
        source: node_id()?.clone(),
        destination,
        body,
    };
    send(&message)?;
    anyhow::Ok(())
}

fn send(message: &Message) -> anyhow::Result<()> {
    let mut stdout = stdout().lock();
    serde_json::to_writer(&mut stdout, message)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    anyhow::Ok(())
}

fn node_id() -> anyhow::Result<&'static String> {
    NODE_ID
        .get()
        .context("initial message should have been received")
}

fn all_node_ids() -> anyhow::Result<&'static Vec<String>> {
    ALL_NODE_IDS
        .get()
        .context("initial message should have been received")
}

fn next_message_id() -> MessageId {
    NEXT_MESSAGE_ID.fetch_add(1, Ordering::Relaxed)
}

fn broadcast_messages_received() -> std::sync::MutexGuard<'static, HashSet<usize>> {
    BROADCAST_MESSAGES_RECEIVED
        .lock()
        .expect("mutex should be lockable")
}

fn neighbors() -> anyhow::Result<&'static Vec<String>> {
    NEIGHBORS
        .get()
        .context("initial message should have been received")
}

fn spawn_stdin_thread(
    sender: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<thread::JoinHandle<anyhow::Result<(), anyhow::Error>>, io::Error> {
    let task = move || -> anyhow::Result<()> {
        for message in io::stdin().lock().lines() {
            sender.send(message?)?;
        }
        anyhow::Ok(())
    };
    thread::Builder::new().name("stdin".to_owned()).spawn(task)
}
