#![allow(dead_code)]

use indexmap::map::IndexMap;
use nom::{
    self,
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::multispace0,
    combinator::{map, value},
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, separated_pair},
    IResult,
};

/// Representation of a node in the json tree
#[derive(Debug, PartialEq, Clone)]
enum JsonNode<'a> {
    Object(Box<IndexMap<&'a str, JsonNode<'a>>>),
    Array(Box<Vec<JsonNode<'a>>>),
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
}

/// Parsing a full json will result in a JsonNode and should consume all the input
fn parse_json(json: &str) -> IResult<&str, JsonNode> {
    alt((
        parse_object,
        parse_array,
        parse_number,
        parse_string,
        parse_boolean,
        parse_null,
    ))(json)
}

fn parse_object(json: &str) -> IResult<&str, JsonNode> {
    map(
        // An object is delimited by {}
        delimited(
            tag("{"),
            // and contains a list of entries separated by comma (','), optionally empty
            separated_list0(
                tag(","),
                // each entry is made of two parts: key and value, separated by colon (':')
                separated_pair(
                    delimited(multispace0, parse_string_inner, multispace0),
                    tag(":"),
                    delimited(multispace0, parse_json, multispace0),
                ),
            ),
            tag("}"),
        ),
        |v| JsonNode::Object(Box::new(v.into_iter().collect())),
    )(json)
}

fn parse_array(json: &str) -> IResult<&str, JsonNode> {
    map(
        // An array is delimited by []
        delimited(
            tag("["),
            // and contains a list of entries separated by comma (','), optionally empty
            separated_list0(delimited(multispace0, tag(","), multispace0), parse_json),
            tag("]"),
        ),
        |v| JsonNode::Array(Box::new(v)),
    )(json)
}

fn parse_number(json: &str) -> IResult<&str, JsonNode> {
    // We can reuse the parser already built into nom!
    map(double, |n| JsonNode::Number(n))(json)
}

/// Parses a string and returns it "raw", without building a JsonNode
fn parse_string_inner(json: &str) -> IResult<&str, &str> {
    // A string is delimited by quote marks. Here we do not handle unicode or escape characters,
    // but take a lookt at https://github.com/rust-bakery/nom/blob/main/examples/string.rs
    delimited(tag("\""), take_until("\""), tag("\""))(json)
}

/// Parses a string and wraps it into a JsonNode
fn parse_string(json: &str) -> IResult<&str, JsonNode> {
    map(parse_string_inner, |s: &str| JsonNode::String(s))(json)
}

fn parse_boolean(json: &str) -> IResult<&str, JsonNode> {
    // A boolean is the literal true or false
    alt((
        value(JsonNode::Boolean(true), tag("true")),
        value(JsonNode::Boolean(false), tag("false")),
    ))(json)
}

fn parse_null(json: &str) -> IResult<&str, JsonNode> {
    // Simplest case: a literal value
    value(JsonNode::Null, tag("null"))(json)
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use nom::error::make_error;

    use crate::{
        parse_array, parse_boolean, parse_null, parse_number, parse_object, parse_string, JsonNode,
    };

    #[test]
    fn can_parse_null() {
        assert_eq!(Ok(("", JsonNode::Null)), parse_null("null"));

        assert_eq!(
            parse_null("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn can_parse_boolean() {
        assert_eq!(Ok(("", JsonNode::Boolean(true))), parse_boolean("true"));
        assert_eq!(Ok(("", JsonNode::Boolean(false))), parse_boolean("false"));

        assert_eq!(
            parse_boolean("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn can_parse_string() {
        assert_eq!(Ok(("", JsonNode::String("abc"))), parse_string("\"abc\""));

        assert_eq!(
            parse_string("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn can_parse_numbers() {
        assert_eq!(Ok(("", JsonNode::Number(42f64))), parse_number("42"));
        assert_eq!(Ok(("", JsonNode::Number(1.2f64))), parse_number("1.2"));
        assert_eq!(Ok(("", JsonNode::Number(1.3e4f64))), parse_number("1.3e4"));
        assert_eq!(Ok(("", JsonNode::Number(0.14f64))), parse_number(".14"));

        assert_eq!(
            parse_string("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn can_parse_array() {
        assert_eq!(
            Ok(("", JsonNode::Array(Box::new(Vec::new())))),
            parse_array("[]")
        );
        assert_eq!(
            Ok(("", JsonNode::Array(Box::new(vec![JsonNode::Boolean(true)])))),
            parse_array("[true]")
        );
        assert_eq!(
            Ok((
                "",
                JsonNode::Array(Box::new(vec![
                    JsonNode::Boolean(false),
                    JsonNode::Null,
                    JsonNode::Boolean(false)
                ]))
            )),
            parse_array("[false, null, false]")
        );

        assert_eq!(
            parse_array("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn can_parse_objects() {
        assert_eq!(
            Ok(("", JsonNode::Object(Box::new(IndexMap::new())))),
            parse_object("{}")
        );
        assert_eq!(
            Ok((
                "",
                JsonNode::Object(Box::new(
                    vec![("b", JsonNode::Boolean(false))].into_iter().collect()
                ))
            )),
            parse_object("{\"b\": false}")
        );
        assert_eq!(
            Ok((
                "",
                JsonNode::Object(Box::new(
                    vec![("a", JsonNode::String("x")), ("b", JsonNode::Boolean(true)),]
                        .into_iter()
                        .collect()
                ))
            )),
            parse_object("{\"a\": \"x\", \"b\": true}")
        );

        assert_eq!(
            parse_object("something"),
            Err(nom::Err::Error(make_error(
                "something",
                nom::error::ErrorKind::Tag
            )))
        );
    }
}
