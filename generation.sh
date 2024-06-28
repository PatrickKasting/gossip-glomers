#!/usr/bin/env bash
cargo build && ./maelstrom/maelstrom test -w unique-ids --bin ./target/debug/gossip_glomers --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition