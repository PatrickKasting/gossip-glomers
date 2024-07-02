mod message;
mod node;

use std::{
    io::{self, BufRead},
    thread,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (stdin_sender, stdin_receiver) = tokio::sync::mpsc::unbounded_channel();
    let stdin_thread = spawn_stdin_thread(stdin_sender)?;

    node::run(stdin_receiver).await;

    stdin_thread
        .join()
        .expect("thread should be finished because the receiver is closed")?;
    anyhow::Ok(())
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
