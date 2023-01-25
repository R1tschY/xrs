use std::borrow::Cow;
use std::fmt::{Formatter, Write};

use serde::de::{Error, MapAccess, SeqAccess, Unexpected, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(datetime)]
use time::{format_description::well_known::Iso8601, OffsetDateTime};

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename = "dateTime.iso8601")]
pub struct DateTimeRawIso8601<'a>(Cow<'a, str>);

impl<'a> DateTimeRawIso8601<'a> {
    pub fn new(date_time: impl Into<Cow<'a, str>>) -> Self {
        Self(date_time.into())
    }
}

#[cfg(datetime)]
pub struct DateTimeIso8601(OffsetDateTime);

#[cfg(datetime)]
impl<'de> Deserialize<'de> for DateTimeIso8601 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Iso8601Visitor;

        impl<'de> Visitor<'de> for Iso8601Visitor {
            type Value = DateTimeIso8601;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("ISO 8601 date time string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                OffsetDateTime::parse(&v, &Iso8601::PARSING)
                    .map(DateTimeIso8601)
                    .map_err(|err| E::custom(err))
            }
        }

        deserializer.deserialize_newtype_struct("dateTime.iso8601", Iso8601Visitor)
    }
}

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    Int(i32),
    Boolean(bool),
    String(Cow<'a, str>),
    Double(f64),
    DateTimeIso8601(DateTimeRawIso8601<'a>),
    Base64(Vec<u8>),
    Struct(Vec<(Cow<'a, str>, Value<'a>)>),
    Array(Vec<Value<'a>>),
    Nil,
}

// Deserialize

struct ValueDeserializeVisitor;

impl<'de> Visitor<'de> for ValueDeserializeVisitor {
    type Value = Value<'de>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "an XML-RPC value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Boolean(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v as i32))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v as i32))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v as i32))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match i32::try_from(v) {
            Ok(v) => Ok(Value::Int(v)),
            Err(_) => Err(Error::invalid_value(
                Unexpected::Signed(v),
                &"signed 32-bit integer value",
            )),
        }
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v as i32))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v as i32))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match i32::try_from(v) {
            Ok(v) => Ok(Value::Int(v)),
            Err(_) => Err(Error::invalid_value(
                Unexpected::Unsigned(v as u64),
                &"signed 32-bit integer value",
            )),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match i32::try_from(v) {
            Ok(v) => Ok(Value::Int(v)),
            Err(_) => Err(Error::invalid_value(
                Unexpected::Unsigned(v),
                &"signed 32-bit integer value",
            )),
        }
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Double(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Double(v))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(Cow::Borrowed(v)))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(Cow::Owned(v)))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Base64(v.to_vec()))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Nil)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Nil)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut res: Vec<Value<'de>> = vec![];
        while let Some(elem) = seq.next_element()? {
            res.push(elem);
        }
        Ok(Value::Array(res))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut res: Vec<(Cow<'de, str>, Value<'de>)> = vec![];
        while let Some((name, value)) = map.next_entry()? {
            res.push((name, value));
        }
        Ok(Value::Struct(res))
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Value<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueDeserializeVisitor)
    }
}

// Serialize

impl<'de> Serialize for Value<'de> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Int(v) => serializer.serialize_i32(*v),
            Value::Boolean(v) => serializer.serialize_bool(*v),
            Value::String(v) => serializer.serialize_str(v.as_ref()),
            Value::Double(v) => serializer.serialize_f64(*v),
            Value::DateTimeIso8601(_v) => {
                todo!()
            }
            Value::Base64(v) => serializer.serialize_bytes(v),
            Value::Struct(v) => {
                let mut ser = serializer.serialize_map(Some(v.len()))?;
                for elem in v.iter() {
                    ser.serialize_entry(&elem.0, &elem.1)?;
                }
                ser.end()
            }
            Value::Array(v) => {
                let mut ser = serializer.serialize_seq(Some(v.len()))?;
                for elem in v.iter() {
                    ser.serialize_element(&elem)?;
                }
                ser.end()
            }
            Value::Nil => serializer.serialize_unit(),
        }
    }
}
