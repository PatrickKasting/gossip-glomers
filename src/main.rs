mod message;
mod node;

use std::{
    io::{self, BufRead},
    thread,
};

use anyhow::Context;

use message::Message;
use node::Node;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (stdin_sender, mut stdin_receiver) = tokio::sync::mpsc::unbounded_channel();
    let stdin_thread = spawn_stdin_thread(stdin_sender)?;
    let mut node = node(&mut stdin_receiver).await?;
    while let Some(message) = stdin_receiver.recv().await {
        node.handle(message)?;
    }
    stdin_thread
        .join()
        .expect("thread should be finished because the receiver is closed")?;
    anyhow::Ok(())
}

fn spawn_stdin_thread(
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
) -> Result<thread::JoinHandle<anyhow::Result<(), anyhow::Error>>, io::Error> {
    let task = move || -> anyhow::Result<()> {
        for message in io::stdin().lock().lines() {
            let message = serde_json::from_str(&message?)?;
            sender.send(message)?;
        }
        Ok(())
    };
    thread::Builder::new().name("stdin".to_owned()).spawn(task)
}

async fn node(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
) -> anyhow::Result<Node<io::StdoutLock<'static>>, anyhow::Error> {
    let first_message = receiver
        .recv()
        .await
        .context("first message should be readable")?;
    Node::new(io::stdout().lock(), first_message)
}
