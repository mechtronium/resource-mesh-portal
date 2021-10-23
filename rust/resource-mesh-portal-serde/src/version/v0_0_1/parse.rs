use std::convert::TryInto;
use std::str::FromStr;

use nom::{AsChar, InputTakeAtPosition, IResult};
use nom::bytes::complete::{tag, take};
use nom::character::complete::{alpha0, alpha1, anychar, digit0, digit1, one_of, alphanumeric1, multispace0};
use nom::combinator::{not, opt, all_consuming};
use nom::error::{context, ErrorKind, VerboseError};
use nom::multi::{many1, many_m_n, separated_list0, separated_list1};
use nom::sequence::{delimited, preceded, terminated, tuple};
use serde::{Deserialize, Serialize};

use nom::branch::alt;

use anyhow::Error;
use crate::version::v0_0_1::id::{Specific, ResourceType,Version};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

fn any_resource_path_segment<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-')
            && !(char_item == '.')
            && !(char_item == '/')
            && !(char_item == '_')
            && !(char_item.is_alpha() || char_item.is_dec_digit())
        },
        ErrorKind::AlphaNumeric,
    )
}

fn loweralphanumerichyphen1<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-')
                && !((char_item.is_alpha() && char_item.is_lowercase()) || char_item.is_dec_digit())
        },
        ErrorKind::AlphaNumeric,
    )
}

fn domain<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-') &&
            !(char_item == '.') &&
            !((char_item.is_alpha() && char_item.is_lowercase()) || char_item.is_dec_digit())
        },
        ErrorKind::AlphaNumeric,
    )
}

fn not_whitespace<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            (char_item == ' ')
        },
        ErrorKind::AlphaNumeric,
    )
}

pub fn not_whitespace_or_semi<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            (char_item == ' ' || char_item == ';')
        },
        ErrorKind::AlphaNumeric,
    )
}

fn anything_but_single_quote<T>(i: T) -> Res<T, T>
    where
        T: InputTakeAtPosition,
        <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            char_item == '\''
        },
        ErrorKind::AlphaNumeric,
    )
}


fn parse_version_major_minor_patch(input: &str) -> Res<&str, (usize, usize, usize)> {
    context(
        "version_major_minor_patch",
        tuple((
            terminated(digit1, tag(".")),
            terminated(digit1, tag(".")),
            terminated(digit1, not(digit1)),
        )),
    )(input)
        .map(|(next_input, res)| {
            (
                next_input,
                (
                    res.0.parse().unwrap(),
                    res.1.parse().unwrap(),
                    res.2.parse().unwrap(),
                ),
            )
        })
}

pub fn parse_version(input: &str) -> Res<&str, Version> {
    context(
        "version",
        tuple((parse_version_major_minor_patch, opt(preceded(tag("-"), parse_skewer)))),
    )(input)
        .map(|(next_input, ((major, minor, patch), release))| {
            let release = match release
            {
                None => {Option::None}
                Some(s) => {Option::Some(s.to_string())}
            };
            (next_input, Version::new(major, minor, patch, release))
        })
}

fn parse_skewer(input: &str) -> Res<&str, &str> {
    context("skewer-case", loweralphanumerichyphen1)(input)
        .map(|(input, skewer)| (input, skewer))
}

pub fn parse_specific(input: &str) -> Res<&str, Specific> {
    context(
        "specific",
        tuple((
            terminated(domain, tag(":")),
            terminated(loweralphanumerichyphen1, tag(":")),
            terminated(loweralphanumerichyphen1, tag(":")),
            parse_version,
        )),
    )(input)
        .map(|(next_input, (vendor, product, variant, version))| {
            (
                next_input,
                Specific {
                    vendor: vendor.to_string(),
                    product: product.to_string(),
                    variant: variant.to_string(),
                    version: version,
                },
            )
        })
}



pub fn parse_address(input: &str) -> Res<&str, Vec<String>> {
    context(
        "address",
        separated_list1(
            nom::character::complete::char(':'),
            any_resource_path_segment
        ),
    )(input).map( |(next,segments)|{
        let segments: Vec<String> = segments.iter().map( |s| s.to_string() ).collect();
        (next,segments)
    })
}





