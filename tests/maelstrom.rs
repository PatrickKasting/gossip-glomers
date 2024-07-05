use std::{fs, process, str};

use clojure_reader::edn::Edn;
use ordered_float::OrderedFloat;
use regex::Regex;

#[test]
fn echo() {
    let configuration = [
        ["--workload", "echo"],
        ["--node-count", "1"],
        ["--time-limit", "10"],
    ];
    test(&configuration);
}

#[test]
fn unique_ids() {
    let configuration = [
        ["--workload", "unique-ids"],
        ["--time-limit", "30"],
        ["--rate", "1000"],
        ["--node-count", "3"],
        ["--availability", "total"],
        ["--nemesis", "partition"],
    ];
    test(&configuration);
}

#[test]
fn single_node_broadcast() {
    let configuration = [
        ["--workload", "broadcast"],
        ["--node-count", "1"],
        ["--time-limit", "20"],
        ["--rate", "10"],
    ];
    test(&configuration);
}

#[test]
fn multi_node_broadcast() {
    let configuration = [
        ["--workload", "broadcast"],
        ["--node-count", "5"],
        ["--time-limit", "20"],
        ["--rate", "10"],
    ];
    test(&configuration);
}

#[test]
fn fault_tolerant_broadcast() {
    let configuration = [
        ["--workload", "broadcast"],
        ["--node-count", "5"],
        ["--time-limit", "20"],
        ["--rate", "10"],
        ["--nemesis", "partition"],
    ];
    test(&configuration);
}

#[test]
fn efficient_broadcast() {
    let configuration = [
        ["--workload", "broadcast"],
        ["--node-count", "25"],
        ["--time-limit", "20"],
        ["--rate", "100"],
        ["--latency", "100"],
    ];
    let results = test(&configuration);

    let results = clojure_reader::edn::read_string(&results).expect("output should be valid edn");
    let messages_per_operation = edn_double(
        &results,
        [
            Edn::Key(":net"),
            Edn::Key(":servers"),
            Edn::Key(":msgs-per-op"),
        ],
    );
    let median_latency = edn_integer(
        &results,
        [
            Edn::Key(":workload"),
            Edn::Key(":stable-latencies"),
            Edn::Double(OrderedFloat(0.5)),
        ],
    );
    let maximum_latency = edn_integer(
        &results,
        [
            Edn::Key(":workload"),
            Edn::Key(":stable-latencies"),
            Edn::Int(1),
        ],
    );

    assert!(
        messages_per_operation < 30.0,
        "number of messages per operation is too great"
    );
    assert!(median_latency < 400, "median latency is too great");
    assert!(maximum_latency < 600, "maximum latency is too great");
}

fn test(configuration: &[[&str; 2]]) -> String {
    let mut args = vec!["test", "--bin", "./target/debug/gossip_glomers"];
    args.extend(configuration.iter().flatten());

    let mut command = process::Command::new("./maelstrom/maelstrom");
    command.args(args);
    let process::Output { status, stdout, .. } =
        command.output().expect("test should run to completion");
    assert!(status.success(), "test should complete successfully");

    let output = str::from_utf8(&stdout).expect("stdout should be utf-8");
    let results =
        fs::read_to_string(results_path(output)).expect("test results should be readable");
    tagless(&results)
}

fn results_path(stdout: &str) -> &str {
    let path_marker = " jepsen results - jepsen.store Wrote ";
    let (_, right) = stdout
        .split_once(path_marker)
        .expect("output should have line containing path to file with results");
    right
        .lines()
        .next()
        .expect("remainder of line should contain path")
}

fn tagless(edn: &str) -> String {
    let tag_regex = Regex::new(r"#[^\{]*").expect("regex should be valid");
    let tag_endpoints = tag_regex
        .find_iter(edn)
        .flat_map(|mat| [mat.start(), mat.end()]);

    let mut slice_endpoints = vec![0];
    slice_endpoints.extend(tag_endpoints);
    slice_endpoints.push(edn.len());

    slice_endpoints
        .chunks_exact(2)
        .map(|pair| &edn[pair[0]..pair[1]])
        .collect()
}

fn edn_value<'edn, 'path: 'edn>(
    mut edn: &'edn Edn<'edn>,
    path: impl IntoIterator<Item = Edn<'path>>,
) -> &'edn Edn<'edn> {
    for key in path {
        edn = edn.get(&key).expect("edn key should exist");
    }
    edn
}

fn edn_integer<'edn, 'path: 'edn>(
    edn: &'edn Edn<'edn>,
    path: impl IntoIterator<Item = Edn<'path>>,
) -> i64 {
    let Edn::Int(int) = edn_value(edn, path) else {
        panic!("value should be integer");
    };
    *int
}

fn edn_double<'edn, 'path: 'edn>(
    edn: &'edn Edn<'edn>,
    path: impl IntoIterator<Item = Edn<'path>>,
) -> f64 {
    let Edn::Double(OrderedFloat(double)) = edn_value(edn, path) else {
        panic!("value should be double");
    };
    *double
}
