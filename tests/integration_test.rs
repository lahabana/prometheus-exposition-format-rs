extern crate prometheus_exposition_format_rs;

use prometheus_exposition_format_rs::parse_complete;
use prometheus_exposition_format_rs::types::{Err, Metric};
use std::fs;

const PATH: &str = "fixtures";

fn read_fixture(s: &str) -> Result<Vec<Metric>, Err> {
    parse_complete(&fs::read_to_string(s).unwrap())
}

fn assert_file_ok(s: &str) -> Vec<Metric> {
    let res = read_fixture(s);
    assert!(res.is_ok(), "Failed to read file '{}' got: \n{:?}", s, res);
    res.unwrap()
}

fn assert_file_nok(s: &str) -> Err {
    let res = read_fixture(s);
    assert!(
        res.is_err(),
        "Succeeded to read file '{}' when we shouldn't got: \n{:?}",
        s,
        res
    );
    res.unwrap_err()
}

fn files_with_prefix(prefix: &str) -> Vec<String> {
    // This should look simpler
    // It looks inside the fixture folder and filters files that ends with *.prom and start with a prefix
    fs::read_dir(PATH)
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().map_or(false, |s| s == "prom"))
        .filter(|f| {
            f.file_name()
                .map(|s| s.to_str())
                .flatten()
                .unwrap()
                .starts_with(prefix)
        })
        .map(|f| f.to_str().unwrap().to_string())
        .collect()
}

#[test]
fn test_ok_fixture_files() {
    for file_name in files_with_prefix("ok_") {
        assert_file_ok(&file_name);
    }
}

#[test]
fn test_nok_fixture_files() {
    for file_name in files_with_prefix("nok_") {
        assert_file_nok(&file_name);
    }
}
