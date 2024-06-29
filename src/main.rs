mod message;
mod node;

use std::io::{self, BufRead};

use anyhow::{Context, Ok, Result};
use node::Node;

fn main() -> Result<()> {
    let mut stdin = io::stdin().lock().lines();
    let mut stdout = io::stdout().lock();

    let initial_message = stdin.next().context("first message should be readable")??;
    let initial_message = serde_json::from_str(&initial_message)?;
    let mut node = Node::new(&mut stdout, initial_message)?;

    for message in stdin {
        let message = serde_json::from_str(&message?)?;
        node.handle(message)?;
    }

    Ok(())
}
