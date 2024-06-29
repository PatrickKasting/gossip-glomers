mod message;
mod node;

use std::{
    io::{self, BufRead},
    sync::mpsc,
    thread,
};

use anyhow::{Context, Error, Ok, Result};
use message::Message;
use node::Node;

enum Event {
    Message(Message),
    Broadcast(usize, String),
    Shutdown,
}

fn main() -> Result<()> {
    let mut node = node()?;

    let (sender, receiver) = mpsc::channel();

    let stdin = move || -> Result<()> {
        for message in io::stdin().lock().lines() {
            let message = serde_json::from_str(&message?)?;
            sender.send(Event::Message(message))?;
        }
        sender.send(Event::Shutdown)?;
        Ok(())
    };
    let stdin = thread::Builder::new()
        .name("stdin".to_owned())
        .spawn(stdin)?;

    for event in receiver {
        match event {
            Event::Message(message) => node.handle(message)?,
            Event::Broadcast(message, sender) => node.broadcast_to_neighbors(message, &sender)?,
            Event::Shutdown => break,
        }
    }

    stdin
        .join()
        .expect("thread 'stdin' should join with main thread")?;
    Ok(())
}

fn node() -> Result<Node<io::StdoutLock<'static>>, Error> {
    let mut stdin = io::stdin().lock().lines();
    let initial_message = stdin.next().context("first message should be readable")??;
    let initial_message = serde_json::from_str(&initial_message)?;
    Ok(Node::new(io::stdout().lock(), initial_message)?)
}
