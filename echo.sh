#!/usr/bin/env bash
cargo build && ./maelstrom/maelstrom test -w echo --bin ./target/debug/gossip_glomers --node-count 1 --time-limit 10