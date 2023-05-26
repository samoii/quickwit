// Copyright (C) 2023 Quickwit, Inc.
//
// Quickwit is offered under the AGPL v3.0 and as commercial software.
// For commercial licensing, contact us at hello@quickwit.io.
//
// AGPL:
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::net::{IpAddr, Ipv6Addr};
use std::str::FromStr;

use once_cell::sync::OnceCell;
use quickwit_datetime::{parse_date_time_str, parse_timestamp, DateTimeInputFormat};
use serde::{Deserialize, Serialize};
use tantivy::schema::IntoIpv6Addr;

fn get_default_date_time_format() -> &'static [DateTimeInputFormat] {
    static DEFAULT_DATE_TIME_FORMATS: OnceCell<Vec<DateTimeInputFormat>> = OnceCell::new();
    DEFAULT_DATE_TIME_FORMATS
        .get_or_init(|| {
            vec![
                DateTimeInputFormat::Rfc3339,
                DateTimeInputFormat::Rfc2822,
                DateTimeInputFormat::Timestamp,
                DateTimeInputFormat::from_str("%Y-%m-%d %H:%M:%S.%f").unwrap(),
                DateTimeInputFormat::from_str("%Y-%m-%d %H:%M:%S").unwrap(),
                DateTimeInputFormat::from_str("%Y-%m-%d").unwrap(),
                DateTimeInputFormat::from_str("%Y/%m/%d").unwrap(),
            ]
        })
        .as_slice()
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum JsonLiteral {
    Number(serde_json::Number),
    // String is a bit special.
    //
    // It can either mean it was passed as a string by the user (via the es query dsl for
    // instance), or it can mean its type is unknown as it was parsed out of tantivy's query
    // language.
    //
    // We have decided to not make a difference at the moment.
    String(String),
    Bool(bool),
}

pub trait InterpretUserInput<'a>: Sized {
    fn interpret_json(user_input: &'a JsonLiteral) -> Option<Self> {
        match user_input {
            JsonLiteral::Number(number) => Self::interpret_number(number),
            JsonLiteral::String(str_val) => Self::interpret_str(str_val),
            JsonLiteral::Bool(bool_val) => Self::interpret_bool(*bool_val),
        }
    }

    fn interpret_number(_number: &serde_json::Number) -> Option<Self> {
        None
    }

    fn interpret_bool(_bool: bool) -> Option<Self> {
        None
    }
    fn interpret_str(_text: &'a str) -> Option<Self> {
        None
    }

    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<'a> InterpretUserInput<'a> for &'a str {
    fn interpret_str(text: &'a str) -> Option<Self> {
        Some(text)
    }
}

impl<'a> InterpretUserInput<'a> for u64 {
    fn interpret_json(user_input: &JsonLiteral) -> Option<u64> {
        match user_input {
            JsonLiteral::Number(json_number) => json_number.as_u64(),
            JsonLiteral::String(text) => text.parse().ok(),
            JsonLiteral::Bool(_) => None,
        }
    }
}

impl<'a> InterpretUserInput<'a> for i64 {
    fn interpret_json(user_input: &JsonLiteral) -> Option<i64> {
        match user_input {
            JsonLiteral::Number(json_number) => json_number.as_i64(),
            JsonLiteral::String(text) => text.parse().ok(),
            JsonLiteral::Bool(_) => None,
        }
    }
}

// We refuse NaN and infinity.
impl<'a> InterpretUserInput<'a> for f64 {
    fn interpret_json(user_input: &JsonLiteral) -> Option<f64> {
        let val: f64 = match user_input {
            JsonLiteral::Number(json_number) => json_number.as_f64()?,
            JsonLiteral::String(text) => text.parse().ok()?,
            JsonLiteral::Bool(_) => {
                return None;
            }
        };
        if val.is_nan() || val.is_infinite() {
            return None;
        }
        Some(val)
    }
}

impl<'a> InterpretUserInput<'a> for bool {
    fn interpret_bool(b: bool) -> Option<Self> {
        Some(b)
    }
    fn interpret_str(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl<'a> InterpretUserInput<'a> for Ipv6Addr {
    fn interpret_str(text: &str) -> Option<Self> {
        let ip_addr: IpAddr = text.parse().ok()?;
        Some(ip_addr.into_ipv6_addr())
    }
}

impl<'a> InterpretUserInput<'a> for tantivy::DateTime {
    fn interpret_str(text: &str) -> Option<Self> {
        let date_time_formats = get_default_date_time_format();
        if let Ok(datetime) = parse_date_time_str(text, date_time_formats) {
            return Some(datetime);
        }
        // Parsing the normal string formats failed.
        // Maybe it is actually a timestamp as a string?
        let possible_timestamp = text.parse::<i64>().ok()?;
        parse_timestamp(possible_timestamp).ok()
    }

    fn interpret_number(number: &serde_json::Number) -> Option<Self> {
        let possible_timestamp = number.as_i64()?;
        parse_timestamp(possible_timestamp).ok()
    }
}

#[cfg(test)]
mod tests {
    use tantivy::DateTime;
    use time::macros::datetime;

    use crate::json_literal::InterpretUserInput;
    use crate::JsonLiteral;

    #[test]
    fn test_interpret_datetime_simple_date() {
        let dt_opt = DateTime::interpret_json(&JsonLiteral::String("2023-05-25".to_string()));
        let expected_datetime = datetime!(2023-05-25 00:00 UTC);
        assert_eq!(dt_opt, Some(DateTime::from_utc(expected_datetime)));
    }

    #[test]
    fn test_interpret_datetime_fractional_millis() {
        let dt_opt =
            DateTime::interpret_json(&JsonLiteral::String("2023-05-25 10:20:11.322".to_string()));
        let expected_datetime = datetime!(2023-05-25 10:20:11.322 UTC);
        assert_eq!(dt_opt, Some(DateTime::from_utc(expected_datetime)));
    }

    #[test]
    fn test_interpret_datetime_unix_timestamp_as_string() {
        let dt_opt = DateTime::interpret_json(&JsonLiteral::String("1685086013".to_string()));
        let expected_datetime = datetime!(2023-05-26 07:26:53 UTC);
        assert_eq!(dt_opt, Some(DateTime::from_utc(expected_datetime)));
    }

    #[test]
    fn test_interpret_datetime_unix_timestamp_as_number() {
        let dt_opt = DateTime::interpret_json(&JsonLiteral::Number(1685086013.into()));
        let expected_datetime = datetime!(2023-05-26 07:26:53 UTC);
        assert_eq!(dt_opt, Some(DateTime::from_utc(expected_datetime)));
    }
}
