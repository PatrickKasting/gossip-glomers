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

use crate::message::{self, Body, Message, Request, Response};

pub type Id = String;
pub type Guid = usize;
pub type BroadcastMessage = usize;

type RequestHandle = tokio::task::JoinHandle<anyhow::Result<()>>;

static ID: OnceLock<Id> = OnceLock::new();
static ALL_IDS: OnceLock<Vec<Id>> = OnceLock::new();
static NEXT_MESSAGE_ID: AtomicUsize = AtomicUsize::new(0);
static ONGOING_REQUESTS: Lazy<Mutex<HashMap<message::Id, RequestHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static NEXT_GUID: AtomicUsize = AtomicUsize::new(usize::MAX);

static BROADCAST_MESSAGES_RECEIVED: Lazy<Mutex<HashSet<BroadcastMessage>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

static NEIGHBORS: OnceLock<Vec<Id>> = OnceLock::new();

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
        Body::Request { request, id } => handle_request(request, source, id),
        Body::Response {
            response,
            id,
            request_id,
        } => handle_response(&response, &source, id, request_id),
    }
}

fn handle_request(request: Request, source: Id, request_id: message::Id) -> anyhow::Result<()> {
    let response_body = Body::Response {
        id: next_message_id(),
        request_id,
        response: response(request, &source)?,
    };
    let response_message = Message {
        source: id()?.clone(),
        destination: source,
        body: response_body,
    };
    send_message(&response_message)?;
    anyhow::Ok(())
}

fn response(request: Request, source: &Id) -> anyhow::Result<Response> {
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

            ID.get_or_init(|| node_id);
            ALL_IDS.get_or_init(|| node_ids);
            NEXT_GUID.store(index, Ordering::Relaxed);

            anyhow::Ok(Response::Init)
        }
        Request::Echo { echo } => anyhow::Ok(Response::Echo { echo }),
        Request::Generate => {
            let number_of_nodes = all_ids()?.len();
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
                .remove(id()?)
                .context("node id should appear as key in received topology")?;
            NEIGHBORS.get_or_init(|| neighbors);
            anyhow::Ok(Response::Topology)
        }
    }
}

fn send_request_multiple_destinations(
    request: &Request,
    destinations: impl IntoIterator<Item = Id>,
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
    request_id: message::Id,
    destination: Id,
) -> anyhow::Result<()> {
    let body = Body::Request {
        request,
        id: request_id,
    };
    let message = Message {
        source: id()?.clone(),
        destination,
        body,
    };

    let mut interval = tokio::time::interval(Duration::from_millis(REPEAT_REQUEST_INTERVAL_MILLIS));
    loop {
        interval.tick().await;
        send_message(&message)?;
    }
}

fn handle_response(
    _response: &Response,
    _source: &Id,
    _response_id: message::Id,
    request_id: message::Id,
) -> anyhow::Result<()> {
    stop_ongoing_request_if_exists(request_id);
    anyhow::Ok(())
}

fn stop_ongoing_request_if_exists(request_id: message::Id) {
    if let Some(task) = ongoing_requests().remove(&request_id) {
        task.abort();
    }
}

fn send_message(message: &Message) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, message)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    anyhow::Ok(())
}

fn id() -> anyhow::Result<&'static Id> {
    ID.get()
        .context("initial message with node id should have been received")
}

fn all_ids() -> anyhow::Result<&'static Vec<Id>> {
    ALL_IDS
        .get()
        .context("initial message with all node ids should have been received")
}

fn next_message_id() -> message::Id {
    NEXT_MESSAGE_ID.fetch_add(1, Ordering::Relaxed)
}

fn ongoing_requests() -> std::sync::MutexGuard<'static, HashMap<message::Id, RequestHandle>> {
    ONGOING_REQUESTS
        .lock()
        .expect("ongoing-requests mutex should be lockable")
}

fn broadcast_messages_received() -> std::sync::MutexGuard<'static, HashSet<BroadcastMessage>> {
    BROADCAST_MESSAGES_RECEIVED
        .lock()
        .expect("broadcast-messages mutex should be lockable")
}

fn neighbors() -> anyhow::Result<&'static Vec<Id>> {
    NEIGHBORS
        .get()
        .context("topology message with neighbors should have been received")
}
