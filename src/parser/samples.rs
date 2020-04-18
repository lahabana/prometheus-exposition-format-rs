use crate::parser::common::token_parser;
#[cfg(test)]
use assert_approx_eq::assert_approx_eq;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, line_ending, none_of, space1};
use nom::combinator::{map, map_opt, map_res, opt, value};
#[cfg(test)]
use nom::error::ErrorKind;
use nom::multi::{fold_many0, separated_list};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
#[cfg(test)]
use nom::Err::Error;
use nom::IResult;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct SampleEntry {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp_ms: Option<i64>,
}

fn timestamp_parser(i: &str) -> IResult<&str, i64> {
    map_opt(is_not("\n "), |x: &str| x.parse::<i64>().ok())(i)
}

/// Parse a floating point value similar to [Go's strconv.ParseFloat](https://golang.org/pkg/strconv/#ParseFloat)
/// It's all explained in the [Prometheus exposition format doc](https://prometheus.io/docs/instrumenting/exposition_formats/#comments-help-text-and-type-information)
fn value_parser(i: &str) -> IResult<&str, f64> {
    alt((
        value(std::f64::NAN, tag("NaN")),
        value(std::f64::INFINITY, tag("+Inf")),
        value(std::f64::NEG_INFINITY, tag("-Inf")),
        map_res(is_not("\n "), |x: &str| x.parse::<f64>()),
    ))(i)
}

fn tag_value_parser(i: &str) -> IResult<&str, String> {
    delimited(
        char('\"'),
        fold_many0(
            alt((
                preceded(
                    char('\\'),
                    alt((
                        value('\n', char('n')),
                        value('\"', char('\"')),
                        value('\\', char('\\')),
                    )),
                ),
                none_of("\n\"\\"),
            )),
            String::new(),
            |mut acc, item| {
                acc.push(item);
                acc
            },
        ),
        char('\"'),
    )(i)
}

fn labels_parser(i: &str) -> IResult<&str, HashMap<String, String>> {
    let list_parser = terminated(
        separated_list(
            char(','),
            separated_pair(token_parser, char('='), tag_value_parser),
        ),
        opt(char(',')),
    );
    let list_parser = map(
        list_parser,
        |l: Vec<(String, String)>| -> HashMap<String, String> { l.into_iter().collect() },
    );

    map(opt(delimited(char('{'), list_parser, char('}'))), |v| {
        v.unwrap_or(HashMap::new())
    })(i)
}

/// Parse a metric sample according to the [exposition format](https://prometheus.io/docs/instrumenting/exposition_formats/#text-format-example).
///
/// # Arguments
///
/// `i` - A input string to parse
///
/// # Example
///
/// ```
/// use prometheus_exposition_format_rs::parser::samples::parse_sample;
/// let res = parse_sample("http_requests_total{method=\"post\",code=\"200\"} 1027 1395066363000\n").unwrap();
///
/// assert_eq!("http_requests_total", res.1.name);
/// assert_eq!("post", res.1.labels["method"]);
/// ```
pub fn parse_sample(i: &str) -> IResult<&str, SampleEntry> {
    let (input, (name, labels, value, timestamp_ms)) = terminated(
        tuple((
            token_parser,
            labels_parser,
            preceded(space1, value_parser),
            opt(preceded(space1, timestamp_parser)),
        )),
        line_ending,
    )(i)?;

    Ok((
        input,
        SampleEntry {
            name,
            labels,
            value,
            timestamp_ms,
        },
    ))
}

#[test]
fn test_timestamp_parser() {
    assert_eq!(timestamp_parser(""), Err(Error(("", ErrorKind::IsNot))));
    assert_eq!(
        timestamp_parser("foobar"),
        Err(Error(("foobar", ErrorKind::MapOpt)))
    );
    assert_eq!(timestamp_parser("1234"), Ok(("", 1234)));
    assert_eq!(timestamp_parser("1234 foo"), Ok((" foo", 1234)));
    assert_eq!(timestamp_parser("-1234 foo"), Ok((" foo", -1234)));
}

#[test]
fn test_tag_value_parser() {
    // Empty string
    assert_eq!(tag_value_parser("\"\""), Ok(("", "".to_string())));
    // Simple string
    assert_eq!(tag_value_parser("\"abc\""), Ok(("", "abc".to_string())));
    // Doesn't consume trailing
    assert_eq!(tag_value_parser("\"abc\"aa"), Ok(("aa", "abc".to_string())));
    // Unescapes escaped "
    assert_eq!(tag_value_parser("\"\\\"\""), Ok(("", "\"".to_string())));
    // Unescapes escaped line break
    assert_eq!(tag_value_parser("\"\\n\""), Ok(("", "\n".to_string())));
    // Unescapes escaped \
    assert_eq!(tag_value_parser("\"\\\\\""), Ok(("", "\\".to_string())));
    // Fails with unescaped line break
    assert_eq!(
        tag_value_parser("\"\n\""),
        Err(Error(("\n\"", ErrorKind::Char)))
    );
    // Complex value from the doc
    assert_eq!(
        tag_value_parser("\"C:\\\\DIR\\\\FILE.TXT\""),
        Ok(("", "C:\\DIR\\FILE.TXT".to_string()))
    );
    // Complex value from the doc
    assert_eq!(
        tag_value_parser("\"Cannot find file:\\n\\\"FILE.TXT\\\"\""),
        Ok(("", "Cannot find file:\n\"FILE.TXT\"".to_string()))
    );
}

#[cfg(test)]
fn vec_to_hashmap(vec: Vec<(&str, &str)>) -> HashMap<String, String> {
    vec.into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[test]
fn test_labels_parser() {
    let assert_labels = |s, vec: Vec<(&str, &str)>| {
        assert_eq!(labels_parser(s), Ok(("", vec_to_hashmap(vec))));
    };

    // Empty labels
    assert_eq!(labels_parser("{}"), Ok(("", HashMap::new())));
    // Empty string
    assert_eq!(labels_parser(""), Ok(("", HashMap::new())));
    // Prefixed
    assert_eq!(labels_parser("d{}"), Ok(("d{}", HashMap::new())));
    // No quotes on label
    assert_eq!(labels_parser("{he=e}"), Ok(("{he=e}", HashMap::new())));
    // A simple label
    assert_labels("{hello=\"how are you?\"}", vec![("hello", "how are you?")]);
    // Multiple labels
    assert_labels("{a=\"b\",c=\"d\"}", vec![("a", "b"), ("c", "d")]);
    // When there's a trailing comma
    assert_labels("{a=\"b\",c=\"d\",}", vec![("a", "b"), ("c", "d")]);
}

#[test]
fn test_value_parser() {
    assert_eq!(value_parser("1027"), Ok(("", 1027f64)));
    assert_eq!(value_parser("1027 ee"), Ok((" ee", 1027f64)));
    assert_eq!(value_parser("1027\nee"), Ok(("\nee", 1027f64)));
    assert_eq!(value_parser("ee"), Err(Error(("ee", ErrorKind::MapRes))));
    assert_eq!(value_parser("+Inf"), Ok(("", std::f64::INFINITY)));
    assert_eq!(value_parser("-Inf"), Ok(("", std::f64::NEG_INFINITY)));
    assert!(value_parser("NaN").unwrap().1.is_nan());
    assert_approx_eq!(value_parser("2.00").unwrap().1, 2f64);
    assert_approx_eq!(value_parser("1e-3").unwrap().1, 0.001);
    assert_approx_eq!(value_parser("123.3412312312").unwrap().1, 123.3412312312);
    assert_approx_eq!(value_parser("1.458255915e9").unwrap().1, 1.458255915e9);
}

#[cfg(test)]
fn assert_sample(
    res: SampleEntry,
    name: &str,
    labels: Vec<(&str, &str)>,
    value: f64,
    timestamp: Option<i64>,
) {
    assert_eq!(
        res.name,
        name.to_string(),
        "sample name is different {:?}",
        res
    );
    assert_eq!(
        res.labels,
        vec_to_hashmap(labels),
        "labels are different {:?}",
        res
    );

    // Ensure we have similar floats considering the extremes and a good epsilon
    assert!(
        res.value.is_sign_positive() == value.is_sign_positive()
            && res.value.is_infinite() == value.is_infinite()
            && res.value.is_nan() == res.value.is_nan(),
        "float non similar actual:{} expected:{}",
        res.value,
        value
    );
    if value.is_finite() {
        assert_approx_eq!(res.value, value);
    }
    assert_eq!(res.timestamp_ms, timestamp, "Timestamps differ {:?}", res);
}

#[cfg(test)]
fn assert_sample_parser(
    s: &str,
    left: &str,
    name: &str,
    labels: Vec<(&str, &str)>,
    value: f64,
    timestamp: Option<i64>,
) {
    let res = parse_sample(s);
    assert!(res.is_ok(), "{:?}", res);
    let res = res.unwrap();
    assert_eq!(res.0, left, "Not the same left string");
    assert_sample(res.1, name, labels, value, timestamp);
}

#[test]
fn test_parse_sample_parser() {
    // Examples from the doc https://prometheus.io/docs/instrumenting/exposition_formats/#text-format-example
    assert_sample_parser(
        "http_requests_total{method=\"post\",code=\"200\"} 1027 1395066363000\n",
        "",
        "http_requests_total",
        vec![("method", "post"), ("code", "200")],
        1027f64,
        Option::Some(1395066363000i64),
    );
    assert_sample_parser(
        "http_requests_total{method=\"post\",code=\"400\"}    3 1395066363000\n",
        "",
        "http_requests_total",
        vec![("method", "post"), ("code", "400")],
        3f64,
        Option::Some(1395066363000i64),
    );
    assert_sample_parser("msdos_file_access_time_seconds{path=\"C:\\\\DIR\\\\FILE.TXT\",error=\"Cannot find file:\\n\\\"FILE.TXT\\\"\"} 1.458255915e9\n", "",
                    "msdos_file_access_time_seconds", vec![("path", "C:\\DIR\\FILE.TXT"), ("error", "Cannot find file:\n\"FILE.TXT\"")], 1.458255915e9, None);
    assert_sample_parser(
        "metric_without_timestamp_and_labels 12.47\n",
        "",
        "metric_without_timestamp_and_labels",
        vec![],
        12.47,
        None,
    );
    assert_sample_parser(
        "something_weird{problem=\"division by zero\"} +Inf -3982045\n",
        "",
        "something_weird",
        vec![("problem", "division by zero")],
        std::f64::INFINITY,
        Some(-3982045),
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"0.05\"} 24054\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "0.05")],
        24054f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"0.1\"} 33444\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "0.1")],
        33444f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"0.2\"} 100392\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "0.2")],
        100392f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"0.5\"} 129389\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "0.5")],
        129389f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"1\"} 133988\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "1")],
        133988f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_bucket{le=\"+Inf\"} 144320\n",
        "",
        "http_request_duration_seconds_bucket",
        vec![("le", "+Inf")],
        144320f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_sum 53423\n",
        "",
        "http_request_duration_seconds_sum",
        vec![],
        53423f64,
        None,
    );
    assert_sample_parser(
        "http_request_duration_seconds_count 144320\n",
        "",
        "http_request_duration_seconds_count",
        vec![],
        144320f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds{quantile=\"0.01\"} 3102\n",
        "",
        "rpc_duration_seconds",
        vec![("quantile", "0.01")],
        3102f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds{quantile=\"0.05\"} 3272\n",
        "",
        "rpc_duration_seconds",
        vec![("quantile", "0.05")],
        3272f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds{quantile=\"0.5\"} 4773\n",
        "",
        "rpc_duration_seconds",
        vec![("quantile", "0.5")],
        4773f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds{quantile=\"0.9\"} 9001\n",
        "",
        "rpc_duration_seconds",
        vec![("quantile", "0.9")],
        9001f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds{quantile=\"0.99\"} 76656\n",
        "",
        "rpc_duration_seconds",
        vec![("quantile", "0.99")],
        76656f64,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds_sum 1.7560473e+07\n",
        "",
        "rpc_duration_seconds_sum",
        vec![],
        1.7560473e+07,
        None,
    );
    assert_sample_parser(
        "rpc_duration_seconds_count 2693\n",
        "",
        "rpc_duration_seconds_count",
        vec![],
        2693f64,
        None,
    );

    // With trailing characters
    assert_sample_parser(
        "rpc_duration_seconds_count 2693\nfoo",
        "foo",
        "rpc_duration_seconds_count",
        vec![],
        2693f64,
        None,
    );

    // Fails when there's just a metric name
    assert_eq!(
        parse_sample("metric_without_timestamp_and_labels\n"),
        Err(Error(("\n", ErrorKind::Space)))
    );
    // Fails when no space
    assert_eq!(
        parse_sample("metric_without_timestamp_and_labels1234\n"),
        Err(Error(("\n", ErrorKind::Space)))
    );
    // Fails when no line break
    assert_eq!(
        parse_sample("metric_without_timestamp_and_labels 1234"),
        Err(Error(("", ErrorKind::CrLf)))
    );
}
