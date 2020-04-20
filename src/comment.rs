use crate::common::token_parser;
use crate::types::MetricType;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::not_line_ending;
use nom::character::complete::{newline, space0, space1};
use nom::combinator::{map, opt};
#[cfg(test)]
use nom::error::ErrorKind;
use nom::sequence::{delimited, preceded, tuple};
#[cfg(test)]
use nom::Err::Error;
use nom::IResult;

#[derive(Debug, PartialEq)]
pub enum CommentType<'a> {
    Type(&'a str, MetricType),
    Help(&'a str),
    Other,
}

/// Parse comments that starts with "# TYPE"
fn type_parser(i: &str) -> IResult<&str, (&str, MetricType)> {
    let metric_parser = map(
        opt(preceded(
            space1,
            alt((
                map(tag("counter"), |_| MetricType::Counter),
                map(tag("gauge"), |_| MetricType::Gauge),
                map(tag("histogram"), |_| MetricType::Histogram),
                map(tag("untyped"), |_| MetricType::Untyped),
                map(tag("summary"), |_| MetricType::Summary),
            )),
        )),
        |x| x.unwrap_or(MetricType::Untyped),
    );

    delimited(
        tuple((tag("#"), space1, tag("TYPE"), space1)),
        tuple((token_parser, metric_parser)),
        tuple((space0, newline)),
    )(i)
}

fn other_comment_parser(i: &str) -> IResult<&str, ()> {
    map(delimited(tag("#"), not_line_ending, newline), |_| ())(i)
}

/// Parse comments that starts with "# HELP"
fn help_parser(i: &str) -> IResult<&str, &str> {
    delimited(
        tuple((tag("#"), space1, tag("HELP"), space1)),
        not_line_ending,
        newline,
    )(i)
}

/// Parses a comment and return the different types
/// TODO make help optional
pub fn comment_parser(i: &str) -> IResult<&str, CommentType> {
    alt((
        map(type_parser, |(name, tpe)| CommentType::Type(name, tpe)),
        map(help_parser, |s| CommentType::Help(s)),
        map(other_comment_parser, |_| CommentType::Other),
    ))(i)
}

// TODO can we make this asserts easier to read/write
#[test]
fn test_type_parser() {
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds histogram\n"),
        Ok(("", ("http_request_duration_seconds", MetricType::Histogram)))
    );
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds\n"),
        Ok(("", ("http_request_duration_seconds", MetricType::Untyped)))
    );
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds   \n"),
        Ok(("", ("http_request_duration_seconds", MetricType::Untyped)))
    );
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds   \nfoo"),
        Ok((
            "foo",
            ("http_request_duration_seconds", MetricType::Untyped)
        ))
    );
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds   summary\n"),
        Ok(("", ("http_request_duration_seconds", MetricType::Summary)))
    );
    assert_eq!(
        type_parser("# TYPE http_request_duration_seconds sometype\n"),
        Err(Error(("sometype\n", ErrorKind::Char)))
    );
}

#[test]
fn test_other_comment_parser() {
    assert_eq!(
        other_comment_parser("# TYPE http_request_duration_seconds histogram\n"),
        Ok(("", ()))
    );
    assert_eq!(
        other_comment_parser("# TYPE http_request_duration_seconds histogram\nfoo"),
        Ok(("foo", ()))
    );
    assert_eq!(
        other_comment_parser("#This is a comment and we don't care about it\n"),
        Ok(("", ()))
    );
    assert_eq!(
        other_comment_parser("foo bar\n"),
        Err(Error(("foo bar\n", ErrorKind::Tag)))
    );
}

#[test]
fn test_help_parser() {
    assert_eq!(
        help_parser("# TYPE http_request_duration_seconds histogram\n"),
        Err(Error((
            "TYPE http_request_duration_seconds histogram\n",
            ErrorKind::Tag
        )))
    );
    assert_eq!(
        help_parser("# HELP http_request_duration_seconds histogram\nfoo"),
        Ok(("foo", "http_request_duration_seconds histogram"))
    );
    assert_eq!(
        help_parser("# This is a comment and we don't care about it\n"),
        Err(Error((
            "This is a comment and we don't care about it\n",
            ErrorKind::Tag
        )))
    );
}

#[test]
fn test_comment_parser() {
    assert_eq!(
        comment_parser("_TYPE histogram\n"),
        Err(Error(("_TYPE histogram\n", ErrorKind::Tag)))
    );
    assert_eq!(
        comment_parser("# http_request_duration_seconds histogram\n"),
        Ok(("", CommentType::Other))
    );
    assert_eq!(
        comment_parser("# HELP some info\n"),
        Ok(("", CommentType::Help("some info")))
    );
    assert_eq!(
        comment_parser("# TYPE http_request_duration_seconds histogram\n"),
        Ok((
            "",
            CommentType::Type("http_request_duration_seconds", MetricType::Histogram,)
        ))
    );
}
