use crate::comment::{comment_parser, CommentType};
use crate::common::empty_line_parser;
use crate::samples::{parse_sample, SampleEntry};
use crate::types::{Err, Metric, MetricType, Sample};
use nom::branch::alt;
use nom::combinator::map;
use nom::IResult;
use std::collections::HashMap;

// Restrict this to internal visibility only
pub(crate) mod comment;
pub(crate) mod common;
pub(crate) mod samples;
pub mod types;

#[derive(Debug)]
enum LineType<'a> {
    Empty,
    Sample(SampleEntry<'a>),
    Comment(CommentType<'a>),
}

fn parse_line(input: &str) -> IResult<&str, LineType> {
    alt((
        map(comment_parser, |l| LineType::Comment(l)),
        map(parse_sample, |l| LineType::Sample(l)),
        map(empty_line_parser, |_| LineType::Empty),
    ))(input)
}

struct InputIter<'a>(&'a str);

impl<'a> Iterator for InputIter<'a> {
    type Item = Result<LineType<'a>, Err>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            match parse_line(self.0) {
                Ok(res) => {
                    self.0 = res.0;
                    Some(Ok(res.1))
                }
                Result::Err(err) => Some(Result::Err(Err::from(err))),
            }
        }
    }
}

impl<'a> Into<Metric> for SampleEntry<'a> {
    fn into(self) -> Metric {
        Metric {
            name: self.name.to_string(),
            data_type: MetricType::Untyped,
            samples: vec![self.into()],
        }
    }
}

impl<'a> Into<Sample> for SampleEntry<'a> {
    fn into(self) -> Sample {
        Sample {
            labels: self
                .labels
                .iter()
                .map(|(&k, v)| (k.to_string(), v.to_string()))
                .collect(),
            value: self.value,
            timestamp: self.timestamp_ms,
        }
    }
}

impl Metric {
    fn append_sample_entry(&mut self, s: SampleEntry) {
        assert_eq!(
            s.name, self.name,
            "Names should be equal when calling update on a metric"
        );
        self.push_sample(s.into());
    }
    fn append_type_def(&mut self, s: &str, t: MetricType) {
        assert_eq!(
            s,
            &self.name[..],
            "Names should be equal when calling update on a metric"
        );
        self.data_type = t;
    }
}

fn add_comment<'a, 'b>(map: &mut HashMap<&'a str, Metric>, c: CommentType<'a>) {
    if let CommentType::Type(s, t) = c {
        if let Some(x) = map.get_mut(s) {
            x.append_type_def(s, t);
        } else {
            map.insert(s, Metric::new(s, t));
        }
    }
}

fn add_sample<'a, 'b>(map: &'b mut HashMap<&'a str, Metric>, s: SampleEntry<'a>) {
    if let Some(x) = map.get_mut(s.name) {
        x.append_sample_entry(s);
    } else {
        map.insert(s.name, s.into());
    };
}

/// Parse a string and return a vector of metrics extracted from it.
pub fn parse_complete<'a>(input: &'a str) -> Result<Vec<Metric>, Err> {
    let mut acc: HashMap<&'a str, Metric> = HashMap::new();
    for l in InputIter(input) {
        match l? {
            LineType::Comment(c) => add_comment(&mut acc, c),
            LineType::Sample(s) => add_sample(&mut acc, s),
            LineType::Empty => {}
        };
    }
    let mut res: Vec<Metric> = acc.drain().map(|(_, v)| v).collect();
    // Make the order constant
    res.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(res)
}

#[cfg(test)]
fn assert_metric(m: &Metric, name: &str, tpe: MetricType, samples: Vec<Sample>) {
    assert_eq!(m.name, name, "name {:?}", m);
    assert_eq!(m.data_type, tpe, "type {:?}", m);
    assert_eq!(m.samples, samples);
}

#[test]
fn test_parse_summary() {
    let res = parse_complete(
        r#"
# TYPE chain_account_commits summary
chain_account_commits {quantile="0.5"} 0

# TYPE chain_account_commits summary
chain_account_commits {quantile="0.75"} 123

# TYPE chain_account_commits summary
chain_account_commits {quantile="0.95"} 50
"#,
    )
    .unwrap();
    assert_eq!(res.len(), 1);
    assert_metric(
        &res[0],
        "chain_account_commits",
        MetricType::Summary,
        vec![
            Sample::new(0f64, Option::None, vec!["quantile", "0.5"]),
            Sample::new(123f64, Option::None, vec!["quantile", "0.75"]),
            Sample::new(50f64, Option::None, vec!["quantile", "0.95"]),
        ],
    );
}

#[test]
fn test_parse_complete() {
    let res = parse_complete(
        r#"
# HELP http_requests_total The total number of HTTP requests.
# TYPE http_requests_total counter
http_requests_total{method="post",code="200"} 1027 1395066363000
http_requests_total{method="post",code="400"} 1028 1395066363000

rpc_duration_seconds_count 2693
"#,
    )
    .unwrap();
    assert_eq!(res.len(), 2);
    assert_metric(
        &res[0],
        "http_requests_total",
        MetricType::Counter,
        vec![
            Sample::new(
                1027f64,
                Option::Some(1395066363000),
                vec!["method", "post", "code", "200"],
            ),
            Sample::new(
                1028f64,
                Option::Some(1395066363000),
                vec!["method", "post", "code", "400"],
            ),
        ],
    );
    assert_metric(
        &res[1],
        "rpc_duration_seconds_count",
        MetricType::Untyped,
        vec![Sample::new(2693f64, None, vec![])],
    );
}
