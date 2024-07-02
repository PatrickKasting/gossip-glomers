use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

use anyhow::Context;
use once_cell::sync::Lazy;

use crate::message::{Body, BroadcastMessage, Message, MessageId, NodeId, Request, Response};

type RequestHandle = tokio::task::JoinHandle<anyhow::Result<()>>;

static NODE_ID: OnceLock<NodeId> = OnceLock::new();
static ALL_NODE_IDS: OnceLock<Vec<NodeId>> = OnceLock::new();
static NEXT_MESSAGE_ID: AtomicUsize = AtomicUsize::new(0);
static ONGOING_REQUESTS: Lazy<Mutex<HashMap<MessageId, RequestHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static NEXT_GUID: AtomicUsize = AtomicUsize::new(usize::MAX);

static BROADCAST_MESSAGES_RECEIVED: Lazy<Mutex<HashSet<BroadcastMessage>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

static NEIGHBORS: OnceLock<Vec<NodeId>> = OnceLock::new();

pub async fn run(mut message_receiver: tokio::sync::mpsc::UnboundedReceiver<String>) {
    while let Some(message) = message_receiver.recv().await {
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
    send_message(&response_message)?;
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

            anyhow::Ok(Response::Init)
        }
        Request::Echo { echo } => anyhow::Ok(Response::Echo { echo }),
        Request::Generate => {
            let number_of_nodes = all_node_ids()?.len();
            let guid = NEXT_GUID.fetch_add(number_of_nodes, Ordering::Relaxed);
            anyhow::Ok(Response::Generate { id: guid })
        }
        Request::Broadcast { message } => {
            if broadcast_messages_received().insert(message) {
                let destinations = neighbors()?
                    .iter()
                    .filter(|&neighbor| neighbor != source)
                    .cloned();
                send_request_multiple_destinations(&Request::Broadcast { message }, destinations)?;
            };
            anyhow::Ok(Response::Broadcast)
        }
        Request::Read => anyhow::Ok(Response::Read {
            messages: broadcast_messages_received().clone(),
        }),
        Request::Topology { mut topology } => {
            let neighbors = topology
                .remove(node_id()?)
                .context("node id should appear as key in received topology")?;
            NEIGHBORS.get_or_init(|| neighbors);
            anyhow::Ok(Response::Topology)
        }
    }
}

fn handle_response(
    source: NodeId,
    response_id: MessageId,
    request_id: MessageId,
    response: Response,
) -> anyhow::Result<()> {
    stop_ongoing_request_if_exists(request_id);
    anyhow::Ok(())
}

fn stop_ongoing_request_if_exists(request_id: usize) {
    if let Some(task) = ongoing_requests().remove(&request_id) {
        task.abort();
    }
}

fn send_request_multiple_destinations(
    request: &Request,
    destinations: impl IntoIterator<Item = NodeId>,
) -> anyhow::Result<()> {
    for destination in destinations {
        send_request(request.clone(), destination)?;
    }
    anyhow::Ok(())
}

fn send_request(request: Request, destination: String) -> anyhow::Result<()> {
    let request_id = next_message_id();
    let new_task = tokio::spawn(repeat_request(request, request_id, destination));
    let existing_task = ongoing_requests().insert(request_id, new_task);
    assert!(
        existing_task.is_none(),
        "request id should not belong to already existing request"
    );
    anyhow::Ok(())
}

const REPEAT_REQUEST_INTERVAL_MILLIS: u64 = 1000;

async fn repeat_request(
    request: Request,
    id: MessageId,
    destination: NodeId,
) -> anyhow::Result<()> {
    let body = Body::Request { request, id };
    let message = Message {
        source: node_id()?.clone(),
        destination,
        body,
    };

    let mut interval = tokio::time::interval(Duration::from_millis(REPEAT_REQUEST_INTERVAL_MILLIS));
    loop {
        interval.tick().await;
        send_message(&message)?;
    }
}

fn send_message(message: &Message) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
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

fn ongoing_requests() -> std::sync::MutexGuard<'static, HashMap<usize, RequestHandle>> {
    ONGOING_REQUESTS.lock().expect("mutex should be lockable")
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
