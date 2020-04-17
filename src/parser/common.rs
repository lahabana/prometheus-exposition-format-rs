use nom::bytes::complete::{take_while, take_while1};
#[cfg(test)]
use nom::error::ErrorKind;
use nom::sequence::tuple;
#[cfg(test)]
use nom::Err::Error;
use nom::IResult;

fn is_simple(x: char) -> bool {
    x.is_alphabetic() || x == '_' || x == ':'
}

/// Parse a Prometheus token
/// https://prometheus.io/docs/concepts/data_model/#metric-names-and-labels
/// Should match regex: `[a-zA-Z_:][a-zA-Z0-9_:]*`
pub fn token_parser(i: &str) -> IResult<&str, String> {
    let (input, (st, end)) = tuple((
        take_while1(is_simple),
        take_while(|x| is_simple(x) || x.is_alphanumeric()),
    ))(i)?;

    Ok((input, format!("{}{}", st, end)))
}

#[test]
fn test_token_parser() {
    let ok_token = |val: &str| assert_eq!(token_parser(val), Ok(("", val.to_string())));
    ok_token("abc_roo");
    ok_token("http_requests_total");
    ok_token("http_request_duration_seconds_bucket");
    ok_token("__http_request_duration_seconds_bucket");
    ok_token("rpc_duration_seconds_count");
    ok_token("foo_0:3");
    ok_token(":foo");
    assert_eq!(
        token_parser(&"33"),
        Err(Error(("33", ErrorKind::TakeWhile1)))
    );
    assert_eq!(
        token_parser(&")3"),
        Err(Error((")3", ErrorKind::TakeWhile1)))
    );
    assert_eq!(token_parser(&"a("), Ok(("(", "a".to_string())));
}
