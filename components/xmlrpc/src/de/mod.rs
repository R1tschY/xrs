use std::borrow::Cow;
use std::io::BufRead;

use serde::de::value::CowStrDeserializer;
use serde::de::{DeserializeOwned, DeserializeSeed, IntoDeserializer};
use serde::{de, forward_to_deserialize_any};
use serde::{serde_if_integer128, Deserialize};

pub use error::DeError;
use xrs_parser::{Reader, STag, XmlEvent, XmlNsEvent};

use crate::de::cow::{CowStrExt, StrExt};
use crate::de::error::{DeError as Error, DeReason as Reason, DeReason};
use crate::value::Value;
use crate::{Fault, MethodCall, MethodResponse};

mod cow;
mod error;

pub struct Deserializer<'a> {
    reader: Reader<'a>,
    peek: Option<XmlEvent<'a>>,
}

pub fn value_from_str<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::new(Reader::new(s));
    de.next_start_name("value")?;
    T::deserialize(&mut de)
}

pub fn value_from_reader<R: BufRead, T: DeserializeOwned>(mut reader: R) -> Result<T, Error> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    value_from_str(&buf)
}

pub fn method_call_from_str<'de, T: Deserialize<'de>>(
    s: &'de str,
) -> Result<MethodCall<'de, T>, Error> {
    let mut de = Deserializer::new(Reader::new(s));

    de.next_start_name("methodCall")?;

    let mut method_name: Option<Cow<'de, str>> = None;
    let mut params: Option<T> = None;

    loop {
        match de.next()? {
            XmlEvent::STag(stag) if stag.name() == "methodName" => {
                if method_name.is_none() {
                    method_name = Some(de.next_scalar_str()?);
                } else {
                    return Err(de.error(DeReason::Message(
                        "duplicate element: `methodName`".to_string(),
                    )));
                }
            }
            XmlEvent::STag(stag) if stag.name() == "params" => {
                if params.is_none() {
                    params = Some(T::deserialize(ParamsDeserializer { de: &mut de })?);
                } else {
                    return Err(
                        de.error(DeReason::Message("duplicate element: `params`".to_string()))
                    );
                }
            }
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), "methodCall");
                break;
            }
            e if is_ignorable(&e) => continue,
            _ => return Err(de.error(DeReason::ExpectedElement("`methodName` or `params`"))),
        }
    }

    let method_name = method_name
        .ok_or_else(|| de.error(Reason::Message("missing element: `methodName`".to_string())))?;

    let params = match params {
        Some(params) => params,
        None => return Err(de.error(DeReason::Message("missing element: `params`".to_string()))),
    };

    Ok(MethodCall::new(method_name, params))
}

pub fn method_call_from_reader<R: BufRead, T: DeserializeOwned>(
    mut reader: R,
) -> Result<MethodCall<'static, T>, Error> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    method_call_from_str(&buf).map(|ok| ok.into_owned())
}

pub fn method_response_from_str<'de, T: Deserialize<'de>>(
    s: &'de str,
) -> Result<MethodResponse<'de, T>, Error> {
    let mut de = Deserializer::new(Reader::new(s));

    de.next_start_name("methodResponse")?;

    let res = loop {
        match de.next()? {
            XmlEvent::STag(stag) if stag.name() == "params" => {
                de.next_start_name("param")?;
                de.next_start_name("value")?;
                let response = MethodResponse::Success(T::deserialize(&mut de)?);
                de.expect_end("param")?;
                de.expect_end("params")?;
                break response;
            }
            XmlEvent::STag(stag) if stag.name() == "fault" => {
                de.next_start_name("value")?;
                let response = MethodResponse::Fault(Fault::deserialize(&mut de)?);
                de.expect_end("fault")?;
                break response;
            }
            e if is_ignorable(&e) => continue,
            _ => return Err(de.error(DeReason::ExpectedElement("`params` or `fault`"))),
        }
    };

    de.expect_end("methodResponse")?;
    Ok(res)
}

pub fn method_response_from_reader<R: BufRead, T: DeserializeOwned>(
    mut reader: R,
) -> Result<MethodResponse<'static, T>, Error> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    method_response_from_str(&buf).map(|ok| ok.into_owned())
}

fn is_ignorable(evt: &XmlEvent) -> bool {
    match evt {
        XmlEvent::STag(_) | XmlEvent::ETag(_) => false,
        XmlEvent::Characters(c) if !c.as_ref().is_xml_whitespace() => false,
        _ => true,
    }
}

impl<'a> Deserializer<'a> {
    pub fn new(reader: Reader<'a>) -> Self {
        Self { reader, peek: None }
    }

    pub fn from_str(data: &'a str) -> Self {
        Self::new(Reader::new(data))
    }

    fn peek(&mut self) -> Result<&XmlEvent<'a>, Error> {
        if self.peek.is_none() {
            self.peek = Some(self.next()?);
        }
        Ok(self.peek.as_ref().unwrap())
    }

    fn next(&mut self) -> Result<XmlEvent<'a>, Error> {
        if let Some(e) = self.peek.take() {
            return Ok(e);
        }

        while let Some(evt) = self.reader.next()? {
            if matches!(
                &evt,
                XmlEvent::STag(_) | XmlEvent::ETag(_) | XmlEvent::Characters(_)
            ) {
                return Ok(evt);
            }
        }

        Err(self.error(Reason::Eof))
    }

    fn next_ignore_whitespace(&mut self) -> Result<XmlEvent<'a>, Error> {
        if let Some(evt) = self.peek.take() {
            if !matches!(&evt, XmlEvent::Characters(c) if c.as_ref().is_xml_whitespace()) {
                return Ok(evt);
            }
        }

        while let Some(evt) = self.reader.next()? {
            if matches!(&evt, XmlEvent::STag(_) | XmlEvent::ETag(_))
                || matches!(&evt, XmlEvent::Characters(c) if !c.as_ref().is_xml_whitespace())
            {
                return Ok(evt);
            }
        }

        Err(self.error(Reason::Eof))
    }

    fn eat_peek(&mut self) {
        self.peek.take();
    }

    fn next_start(&mut self) -> Result<STag<'a>, Error> {
        match self.next_ignore_whitespace()? {
            XmlEvent::STag(e) => Ok(e),
            _ => Err(self.error(Reason::Start)),
        }
    }

    fn next_start_name(&mut self, name: &'static str) -> Result<STag<'a>, Error> {
        match self.next_ignore_whitespace()? {
            XmlEvent::STag(e) if e.name.as_ref() == name => Ok(e),
            _ => Err(self.error(Reason::ExpectedElement(name))),
        }
    }

    fn next_maybe_start(&mut self) -> Result<Option<STag<'a>>, Error> {
        match self.next_ignore_whitespace()? {
            XmlEvent::STag(e) => Ok(Some(e)),
            XmlEvent::ETag(_) => Ok(None),
            _ => Err(self.error(Reason::Start)),
        }
    }

    /// Consumes Characters with terminating end tag
    fn next_scalar_str(&mut self) -> Result<Cow<'a, str>, Error> {
        let mut result = Cow::<'a, str>::default();
        loop {
            match self.next()? {
                XmlEvent::Characters(chars) => result.push_cow(chars),
                XmlEvent::ETag(_) => return Ok(result),
                _ => return Err(self.error(Reason::NoMarkupExpected)),
            }
        }
    }

    fn expect_end(&mut self, name: &'static str) -> Result<(), Error> {
        match self.next_ignore_whitespace()? {
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), name);
                Ok(())
            }
            _ => Err(self.error(Reason::ExpectedEndElement(name))),
        }
    }

    pub(crate) fn error(&self, reason: Reason) -> Error {
        Error::new(reason, self.reader.cursor_offset())
    }

    pub(crate) fn peek_error(&self, reason: Reason) -> Error {
        Error::new(reason, self.reader.cursor_offset())
    }

    pub(crate) fn fix_position(&self, mut err: Error) -> Error {
        if err.offset() == 0 {
            err.set_offset(self.reader.cursor_offset());
        }
        err
    }

    pub(crate) fn fix_result<'de, V, E>(&self, res: Result<V, E>) -> Result<V, Error>
    where
        E: Into<Error>,
    {
        res.map_err(|err| self.fix_position(err.into()))
    }

    fn set_peek(&mut self, evt: XmlEvent<'a>) {
        debug_assert!(self.peek.is_none());
        self.peek = Some(evt);
    }

    fn read_int<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let result = self.next_scalar_str()?.parse();
        visitor.visit_i32(self.fix_result(result)?)
    }

    fn read_boolean<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        match self.next_scalar_str()?.as_ref() {
            "0" => visitor.visit_bool(false),
            "1" => visitor.visit_bool(true),
            invalid => Err(self.error(Reason::InvalidBoolean(invalid.to_string()))),
        }
    }

    fn read_char<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let result = self.next_scalar_str()?.parse();
        visitor.visit_char::<Error>(self.fix_result(result)?)
    }

    fn read_string<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        match self.next_scalar_str()? {
            Cow::Borrowed(borrowed) => visitor.visit_borrowed_str(borrowed),
            Cow::Owned(owned) => visitor.visit_string(owned),
        }
    }

    fn read_double<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let result = self.next_scalar_str()?.parse();
        visitor.visit_f64(self.fix_result(result)?)
    }

    fn read_date_time_iso8601<V: de::Visitor<'a>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, Error> {
        let text = self.next_scalar_str()?;
        visitor.visit_newtype_struct(text.into_deserializer())
    }

    fn read_base64<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let text = self.next_scalar_str()?;
        let base64 = base64::decode(text.as_ref()).map_err(|err| self.fix_position(err.into()))?;
        visitor.visit_byte_buf(base64)
    }

    fn read_nil<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let text = self.next_scalar_str()?;
        if text.is_empty() {
            visitor.visit_unit()
        } else {
            Err(self.error(Reason::InvalidNil(text.to_string())))
        }
    }

    fn read_struct<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let res = visitor
            .visit_map(StructDeserializer::new(self))
            .map_err(|err| self.fix_position(err))?;
        self.expect_end("struct")?; // TODO: map to `more elements as expected` error
        Ok(res)
    }

    fn read_array<V: de::Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value, Error> {
        let res = visitor
            .visit_seq(ArrayDeserializer::new(self)?)
            .map_err(|err| self.fix_position(err))?;
        self.expect_end("data")?; // TODO: map to `more elements as expected` error
        self.expect_end("array")?;
        Ok(res)
    }

    fn next_type(&mut self, ty: &'static str) -> Result<(), Error> {
        match self.next_ignore_whitespace()? {
            XmlEvent::STag(stag) => {
                if stag.name.as_ref() == ty {
                    Ok(())
                } else {
                    Err(self.error(Reason::WrongType(ty, stag.name.to_string())))
                }
            }
            _ => Err(self.error(Reason::ValueExpected)),
        }
    }
}

macro_rules! deserialize_int {
    ($deserialize:ident => $ty:path, $visit:ident) => {
        fn $deserialize<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let res = match self.next_ignore_whitespace()? {
                XmlEvent::STag(stag) => match stag.name() {
                    "i4" | "int" => {
                        let res = self.next_scalar_str()?.parse::<$ty>();
                        visitor.$visit(self.fix_result(res)?)
                    }
                    _ => Err(self.error(Reason::WrongType("`i4` or `int`", stag.name.to_string()))),
                },
                _ => Err(self.error(Reason::ValueExpected)),
            }?;
            self.expect_end("value")?;
            Ok(res)
        }
    };
}

macro_rules! deserialize_double {
    ($deserialize:ident => $ty:path, $visit:tt) => {
        fn $deserialize<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            self.next_type("double")?;
            let res = self.next_scalar_str()?.parse::<$ty>();
            let res: V::Value = visitor.$visit::<Error>(self.fix_result(res)?)?;
            self.expect_end("value")?;
            Ok(res)
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let res = match self.next_ignore_whitespace()? {
            XmlEvent::STag(stag) => match stag.name() {
                "i4" | "int" => self.read_int(visitor),
                "nil" => self.read_nil(visitor),
                "boolean" => self.read_boolean(visitor),
                "string" => self.read_string(visitor),
                "double" => self.read_double(visitor),
                "base64" => self.read_base64(visitor),
                "struct" => self.read_struct(visitor),
                "array" => self.read_array(visitor),
                "dateTime.iso8601" => self.read_date_time_iso8601(visitor),
                _ => Err(self.error(Reason::UnknownType(stag.name.to_string()))),
            },
            evt @ XmlEvent::Characters(_) => {
                self.set_peek(evt);
                return self.read_string(visitor);
            }
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), "value");
                return visitor.visit_borrowed_str("");
            }
            _ => Err(self.error(Reason::ValueExpected)),
        }?;
        self.expect_end("value")?;
        Ok(res)
    }

    deserialize_int!(deserialize_i8 => i8, visit_i8);
    deserialize_int!(deserialize_i16 => i16, visit_i16);
    deserialize_int!(deserialize_i32 => i32, visit_i32);
    deserialize_int!(deserialize_i64 => i64, visit_i64);
    deserialize_int!(deserialize_u8 => u8, visit_u8);
    deserialize_int!(deserialize_u16 => u16, visit_u16);
    deserialize_int!(deserialize_u32 => u32, visit_u32);
    deserialize_int!(deserialize_u64 => u64, visit_u64);
    deserialize_double!(deserialize_f32 => f32, visit_f32);
    deserialize_double!(deserialize_f64 => f64, visit_f64);

    serde_if_integer128! {
        deserialize_int!(deserialize_i128 => i128, visit_i128);
        deserialize_int!(deserialize_u128 => u128, visit_u128);
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.next_type("boolean")?;
        let res = self.read_boolean(visitor)?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next_ignore_whitespace()? {
            evt @ XmlEvent::STag(_) => {
                self.set_peek(evt);
                self.next_type("string")?;
                let res = self.read_char(visitor)?;
                self.expect_end("value")?;
                Ok(res)
            }
            evt @ XmlEvent::Characters(_) => {
                self.set_peek(evt);
                self.read_char(visitor)
            }
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), "value");
                return visitor.visit_borrowed_str("");
            }
            _ => Err(self.error(Reason::ValueExpected)),
        }
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next_ignore_whitespace()? {
            evt @ XmlEvent::STag(_) => {
                self.set_peek(evt);
                self.next_type("string")?;
                let res = self.read_string(visitor)?;
                self.expect_end("value")?;
                Ok(res)
            }
            evt @ XmlEvent::Characters(_) => {
                self.set_peek(evt);
                self.read_string(visitor)
            }
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), "value");
                return visitor.visit_borrowed_str("");
            }
            _ => Err(self.error(Reason::ValueExpected)),
        }
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.next_type("base64")?;
        let res = self.read_base64(visitor)?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_byte_buf<V>(
        self,
        visitor: V,
    ) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        // TODO: ignore whitespace
        match self.peek()? {
            XmlEvent::STag(stag) if stag.name() == "nil" => {
                self.eat_peek();

                let text = self.next_scalar_str()?;
                if !text.is_empty() {
                    return Err(self.error(Reason::InvalidNil(text.to_string())));
                }

                let res = visitor.visit_none::<Error>()?;
                self.expect_end("value")?;
                Ok(res)
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.next_type("nil")?;
        let res = self.read_nil(visitor)?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        if name == "dateTime.iso8601" {
            self.next_type("dateTime.iso8601")?;
            let res = self.read_date_time_iso8601(visitor)?;
            self.expect_end("value")?;
            Ok(res)
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.next_type("array")?;
        let res = self.read_array(visitor)?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_tuple<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.next_type("struct")?;
        let res = self.read_struct(visitor)?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let res = match self.next_ignore_whitespace()? {
            XmlEvent::STag(stag) => match stag.name.as_ref() {
                "i4" | "int" => {
                    let var = self.next_scalar_str()?;
                    let var: u32 = self.fix_result(var.parse())?;
                    visitor.visit_enum(var.into_deserializer())
                }
                "string" => visitor.visit_enum(self.next_scalar_str()?.into_deserializer()),
                "struct" => {
                    self.next_type("struct")?;
                    visitor
                        .visit_enum(EnumDeserializer::new(self))
                        .map_err(|err| self.fix_position(err))
                }
                _ => Err(self.error(Reason::WrongType(
                    "i4, int, string or struct as enum variant",
                    stag.name.to_string(),
                ))),
            },
            evt @ XmlEvent::Characters(_) => {
                self.set_peek(evt);
                self.read_string(visitor)
            }
            XmlEvent::ETag(etag) => {
                debug_assert_eq!(etag.name(), "value");
                return visitor.visit_borrowed_str("");
            }
            _ => Err(self.error(Reason::ValueExpected)),
        }?;
        self.expect_end("value")?;
        Ok(res)
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let mut depth = 1usize;
        while depth > 0 {
            match self.next()? {
                XmlEvent::STag(_) => depth += 1,
                XmlEvent::ETag(_) => depth -= 1,
                _ => {}
            }
        }
        visitor.visit_unit().map_err(|err| self.fix_position(err))
    }
}

struct StructDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    value: Option<Value<'de>>,
}

impl<'a, 'de> StructDeserializer<'a, 'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, value: None }
    }
}

impl<'a, 'de> de::MapAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        // <member>
        match self.de.next_ignore_whitespace()? {
            XmlEvent::STag(e) if e.name() == "member" => {}
            XmlEvent::ETag(e) => {
                debug_assert_eq!(e.name(), "struct");
                self.de.set_peek(XmlEvent::ETag(e));
                return Ok(None);
            }
            _ => return Err(self.de.error(Reason::ExpectedElement("`member`"))),
        }

        match self.de.next_maybe_start()? {
            // <name>
            Some(stag) if stag.name.as_ref() == "name" => seed
                .deserialize(self.de.next_scalar_str()?.into_deserializer())
                .map(Some),
            // <value>
            Some(stag) if stag.name.as_ref() == "value" => {
                todo!()
            }
            None => Ok(None),
            _ => Err(self.de.error(Reason::ExpectedElement("name or value"))),
        }
    }

    fn next_value_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        let res = if let Some(_value) = self.value.take() {
            todo!()
        } else {
            // <value>
            self.de.next_start_name("value")?;
            seed.deserialize(&mut *self.de)?
        };

        // </member>
        self.de.expect_end("member")?;

        Ok(res)
    }

    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        match self.de.next_ignore_whitespace()? {
            XmlEvent::STag(e) if e.name() == "member" => {}
            XmlEvent::ETag(e) => {
                debug_assert_eq!(e.name(), "struct");
                self.de.set_peek(XmlEvent::ETag(e));
                return Ok(None);
            }
            _ => return Err(self.de.error(Reason::ExpectedElement("`member`"))),
        }

        let res = match self.de.next_start()? {
            stag if stag.name.as_ref() == "name" => {
                let name: K::Value = kseed.deserialize::<CowStrDeserializer<'de, Self::Error>>(
                    self.de.next_scalar_str()?.into_deserializer(),
                )?;

                if self.de.next_start()?.name.as_ref() == "value" {
                    let value: V::Value = vseed.deserialize(&mut *self.de)?;
                    Ok((name, value))
                } else {
                    Err(self.de.error(Reason::ExpectedElement("`value`")))
                }
            }

            stag if stag.name.as_ref() == "value" => {
                let value: V::Value = vseed.deserialize(&mut *self.de)?;

                if self.de.next_start()?.name.as_ref() == "name" {
                    let name = kseed.deserialize::<CowStrDeserializer<'de, Self::Error>>(
                        self.de.next_scalar_str()?.into_deserializer(),
                    )?;
                    Ok((name, value))
                } else {
                    Err(self.de.error(Reason::ExpectedElement("`name`")))
                }
            }

            _ => Err(self.de.error(Reason::ExpectedElement("`name` or `value`"))),
        }?;

        self.de.expect_end("member")?;

        Ok(Some(res))
    }
}

pub struct ArrayDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> ArrayDeserializer<'a, 'de> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de>) -> Result<Self, Error> {
        de.next_start_name("data")?;
        Ok(Self { de })
    }
}

impl<'de, 'a> de::SeqAccess<'de> for ArrayDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.de.next_ignore_whitespace()? {
            XmlEvent::STag(stag) if stag.name.as_ref() == "value" => {
                seed.deserialize(&mut *self.de).map(Some)
            }
            evt @ XmlEvent::ETag(_) => {
                self.de.set_peek(evt);
                Ok(None)
            }
            _ => return Err(self.de.error(Reason::ExpectedElement("value"))),
        }
    }
}

pub struct EnumDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> EnumDeserializer<'a, 'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = VariantDeserializer<'a, 'de>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantDeserializer<'a, 'de>), Error> {
        // <member>
        self.de.next_start_name("member")?;

        match self.de.next_start() {
            // <name>
            Ok(stag) if stag.name.as_ref() == "name" => seed
                .deserialize(self.de.next_scalar_str()?.into_deserializer())
                .map(|res| {
                    (
                        res,
                        VariantDeserializer {
                            de: self.de,
                            value: None,
                        },
                    )
                }),
            // <value>
            Ok(stag) if stag.name.as_ref() == "value" => {
                todo!()
            }
            _ => Err(self.de.error(Reason::ExpectedElement("<name> or <value>"))),
        }
    }
}

pub struct VariantDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    value: Option<Value<'de>>,
}

impl<'de, 'a> de::VariantAccess<'de> for VariantDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Err(Error::new(Reason::NoMarkupExpected, 0))
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

// ParamsDeserializer

struct ParamsDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> de::Deserializer<'de> for ParamsDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(self.de.error(Reason::RootStruct))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.expect_end("params")?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let result = visitor.visit_seq(ParamsSeqDeserializer::new(self.de));
        self.de.expect_end("params")?;
        result
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 bool char str identifier ignored_any enum
        map option string bytes byte_buf
    }
}

struct ParamsSeqDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> ParamsSeqDeserializer<'a, 'de> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for ParamsSeqDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.de.next_ignore_whitespace()? {
            XmlEvent::STag(e) if e.name() == "param" => {
                self.de.next_start_name("value")?;
                let res = seed.deserialize(&mut *self.de)?;
                self.de.expect_end("param")?;
                Ok(Some(res))
            }
            XmlEvent::ETag(e) => {
                debug_assert_eq!(e.name(), "params");
                self.de.set_peek(XmlEvent::ETag(e));
                Ok(None)
            }
            _ => Err(self.de.error(Reason::ExpectedElement("`param`"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;

    use crate::MethodCall;

    use super::*;

    #[test]
    fn full_example_call() {
        let input = r#"<?xml version="1.0"?>
            <methodCall>
                <methodName>http.respond</methodName>
                <params>
                    <param>
                        <value><i4>200</i4></value>
                    </param>
                    <param>
                        <value>
                            <struct>
                                <member>
                                    <name>user-agent</name>
                                    <value><string>xrs</string></value>
                                </member>
                            </struct>
                        </value>
                    </param>
                    <param>
                        <value><string>Hello!</string></value>
                    </param>
                </params>
            </methodCall>"#;

        let call: MethodCall<(i32, HashMap<String, String>, String)> =
            method_call_from_str(input).unwrap();

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("user-agent".to_string(), "xrs".to_string());
        assert_eq!(
            call,
            MethodCall {
                method_name: "http.respond".into(),
                params: (200, headers, "Hello!".to_string())
            }
        )
    }

    mod params_unit {
        use super::*;

        #[test]
        fn empty_params_unit() {
            let input = r#"<?xml version="1.0"?>
                <methodCall>
                    <methodName>xmlrpc.echo</methodName>
                    <params>
                    </params>
                </methodCall>"#;

            let actual: MethodCall<()> = method_call_from_str(input).unwrap();

            assert_eq!(
                actual,
                MethodCall {
                    method_name: "xmlrpc.echo".into(),
                    params: ()
                }
            )
        }
    }

    mod params_tuple {
        use super::*;

        #[test]
        fn empty_params_tuple() {
            let input = r#"<?xml version="1.0"?>
                <methodCall>
                    <methodName>xmlrpc.echo</methodName>
                    <params>
                        <param><value><i4>1</i4></value></param>
                        <param><value><i4>2</i4></value></param>
                    </params>
                </methodCall>"#;

            let actual: MethodCall<(i32, i32)> = method_call_from_str(input).unwrap();

            assert_eq!(
                actual,
                MethodCall {
                    method_name: "xmlrpc.echo".into(),
                    params: (1, 2)
                }
            )
        }
    }

    mod params_seq {
        use super::*;

        #[test]
        fn empty_params_seq() {
            let input = r#"<?xml version="1.0"?>
                <methodCall>
                    <methodName>xmlrpc.echo</methodName>
                    <params>
                    </params>
                </methodCall>"#;

            let actual: MethodCall<Vec<Value>> = method_call_from_str(input).unwrap();

            assert_eq!(
                actual,
                MethodCall {
                    method_name: "xmlrpc.echo".into(),
                    params: vec![]
                }
            )
        }

        #[test]
        fn params_seq() {
            let input = r#"<?xml version="1.0"?>
                    <methodCall>
                        <methodName>xmlrpc.echo</methodName>
                        <params>
                            <param><value><i4>1</i4></value></param>
                            <param><value><i4>2</i4></value></param>
                            <param><value><i4>3</i4></value></param>
                        </params>
                    </methodCall>"#;

            let actual: MethodCall<Vec<Value>> = method_call_from_str(input).unwrap();

            assert_eq!(
                actual,
                MethodCall {
                    method_name: "xmlrpc.echo".into(),
                    params: vec![Value::Int(1), Value::Int(2), Value::Int(3)]
                }
            )
        }
    }

    mod response {
        use super::*;

        #[test]
        fn success() {
            let input = r#"<?xml version="1.0"?>
                <methodResponse>
                    <params>
                        <param>
                            <value>
                                <string>42</string>
                            </value>
                        </param>
                    </params>
                </methodResponse>"#;

            let response: MethodResponse<String> = method_response_from_str(input).unwrap();

            assert_eq!(response, MethodResponse::Success("42".to_string()))
        }

        #[test]
        fn fault() {
            let input = r#"<?xml version="1.0"?>
                <methodResponse>
                    <fault>
                        <value>
                            <struct>
                                <member>
                                    <name>faultCode</name>
                                    <value><int>128</int></value>
                                </member>
                                <member>
                                    <name>faultString</name>
                                    <value><string>something failed</string></value>
                                </member>
                            </struct>
                        </value>
                    </fault>
                </methodResponse>"#;

            let response: MethodResponse<String> = method_response_from_str(input).unwrap();

            assert_eq!(
                response,
                MethodResponse::Fault(Fault {
                    fault_code: 128,
                    fault_string: "something failed".into()
                })
            )
        }
    }

    mod int_value {
        use super::*;

        #[test]
        fn i4() {
            let input = r#"<value><i4>1</i4></value>"#;

            let actual: i32 = value_from_str(input).unwrap();

            assert_eq!(actual, 1)
        }

        #[test]
        fn int() {
            let input = r#"<value><int>1</int></value>"#;

            let actual: i32 = value_from_str(input).unwrap();

            assert_eq!(actual, 1)
        }

        #[test]
        fn plus() {
            let input = r#"<value><int>+1</int></value>"#;

            let actual: i32 = value_from_str(input).unwrap();

            assert_eq!(actual, 1)
        }

        #[test]
        fn minus() {
            let input = r#"<value><int>-1</int></value>"#;

            let actual: i32 = value_from_str(input).unwrap();

            assert_eq!(actual, -1)
        }

        #[test]
        fn leading_zeros() {
            let input = r#"<value><int>-00000009</int></value>"#;

            let actual: i32 = value_from_str(input).unwrap();

            assert_eq!(actual, -9)
        }
    }

    mod string_value {
        use super::*;

        #[test]
        fn string() {
            let input = r#"<value><string>abc</string></value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "abc".to_string())
        }

        #[test]
        fn composed_string() {
            let input = r#"<value><string>a<!--Comment-->b<?php?>c</string></value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "abc".to_string())
        }

        #[test]
        fn implicit_string() {
            let input: &'static str = r#"<value>abc</value>"#;

            let actual: &'static str = value_from_str(input).unwrap();

            assert_eq!(actual, "abc".to_string())
        }

        #[test]
        fn composed_implicit_string() {
            let input = r#"<value>a<!--Comment-->b<?php?>c</value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "abc".to_string())
        }

        #[test]
        fn borrowed_string() {
            let input: &'static str = r#"<value><string>abc</string></value>"#;

            let actual: &'static str = value_from_str(input).unwrap();

            assert_eq!(actual, "abc")
        }

        #[test]
        fn whitespace() {
            let input = "<value><string> \t\n</string></value>";

            let actual: &str = value_from_str(input).unwrap();

            assert_eq!(actual, " \t\n")
        }

        #[test]
        fn empty_string() {
            let input = r#"<value><string><!----><?php?><!----></string></value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "".to_string())
        }

        #[test]
        fn implicit_empty_string() {
            let input = r#"<value></value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "".to_string())
        }

        #[test]
        fn implicit_empty_string_with_comment() {
            let input = r#"<value><!----><?php?><!----></value>"#;

            let actual: String = value_from_str(input).unwrap();

            assert_eq!(actual, "".to_string())
        }

        #[test]
        fn string_as_any() {
            let input = r#"<value><string>abc</string></value>"#;

            let actual: Value = value_from_str(input).unwrap();

            assert_eq!(actual, Value::String("abc".into()))
        }

        #[test]
        fn implicit_string_as_any() {
            let input = r#"<value>abc</value>"#;

            let actual: Value = value_from_str(input).unwrap();

            assert_eq!(actual, Value::String("abc".into()))
        }

        #[test]
        fn implicit_empty_string_as_any() {
            let input = r#"<value></value>"#;

            let actual: Value = value_from_str(input).unwrap();

            assert_eq!(actual, Value::String("".into()))
        }
    }

    mod boolean_value {
        use super::*;

        #[test]
        fn true_() {
            let input = r#"<value><boolean>1</boolean></value>"#;

            let actual: bool = value_from_str(input).unwrap();

            assert_eq!(actual, true)
        }

        #[test]
        fn false_() {
            let input = r#"<value><boolean>0</boolean></value>"#;

            let actual: bool = value_from_str(input).unwrap();

            assert_eq!(actual, false)
        }
    }

    mod double_value {
        use super::*;

        #[test]
        fn integer() {
            let input = r#"<value><double>1</double></value>"#;

            let actual: f64 = value_from_str(input).unwrap();

            assert_eq!(actual, 1.0)
        }

        #[test]
        fn double() {
            let input = r#"<value><double>+0.1234</double></value>"#;

            let actual: f64 = value_from_str(input).unwrap();

            assert_eq!(actual, 0.1234)
        }

        #[test]
        fn float() {
            let input = r#"<value><double>+0.1234</double></value>"#;

            let actual: f32 = value_from_str(input).unwrap();

            assert_eq!(actual, 0.1234)
        }
    }

    mod date_time_value {
        use crate::value::DateTimeRawIso8601;

        use super::*;

        #[test]
        fn valid() {
            let input = r#"<value><dateTime.iso8601>DATETIME</dateTime.iso8601></value>"#;

            let actual: DateTimeRawIso8601 = value_from_str(input).unwrap();

            assert_eq!(actual, DateTimeRawIso8601::new("DATETIME"))
        }

        #[test]
        fn wrong_type() {
            let input = r#"<value><int>1</int></value>"#;

            let actual: Result<DateTimeRawIso8601, Error> = value_from_str(input);

            assert!(matches!(actual, Err(_)));
        }
    }

    mod struct_value {
        use super::*;

        #[test]
        fn empty() {
            #[derive(Deserialize, PartialEq, Debug)]
            struct Struct {}

            let input = r#"<value><struct></struct></value>"#;

            let actual: Struct = value_from_str(input).unwrap();

            assert_eq!(actual, Struct {})
        }

        #[test]
        fn one_member() {
            #[derive(Deserialize, PartialEq, Debug)]
            struct Struct {
                _1: i32,
            };

            let input = r#"
                <value>
                    <struct>
                        <member>
                            <name>_1</name>
                            <value><int>1</int></value>
                        </member>
                    </struct>
                </value>"#;

            let actual: Struct = value_from_str(input).unwrap();

            assert_eq!(actual, Struct { _1: 1 })
        }

        #[test]
        fn two_member() {
            #[derive(Deserialize, PartialEq, Debug)]
            struct Struct {
                _1: i32,
                _2: i32,
            };

            let input = r#"
                <value>
                    <struct>
                        <member>
                            <name>_1</name>
                            <value><int>1</int></value>
                        </member>
                        <member>
                            <name>_2</name>
                            <value><int>2</int></value>
                        </member>
                    </struct>
                </value>"#;

            let actual: Struct = value_from_str(input).unwrap();

            assert_eq!(actual, Struct { _1: 1, _2: 2 })
        }

        #[test]
        fn nested() {
            #[derive(Deserialize, PartialEq, Debug)]
            struct Struct1 {
                inner: Struct2,
            };
            #[derive(Deserialize, PartialEq, Debug)]
            struct Struct2 {};

            let input = r#"
                <value>
                    <struct>
                        <member>
                            <name>inner</name>
                            <value>
                                <struct></struct>
                            </value>
                        </member>
                    </struct>
                </value>"#;

            let actual: Struct1 = value_from_str(input).unwrap();

            assert_eq!(actual, Struct1 { inner: Struct2 {} })
        }
    }

    mod map_value {
        use super::*;

        #[test]
        fn empty() {
            let input = r#"<value><struct></struct></value>"#;

            let actual: HashMap<String, Value> = value_from_str(input).unwrap();

            assert_eq!(actual, HashMap::new())
        }

        #[test]
        fn one_member() {
            let input = r#"
                <value>
                    <struct>
                        <member>
                            <name>_1</name>
                            <value><int>1</int></value>
                        </member>
                    </struct>
                </value>"#;

            let actual: HashMap<String, Value> = value_from_str(input).unwrap();

            let mut expected = HashMap::new();
            expected.insert("_1".to_string(), Value::Int(1));
            assert_eq!(actual, expected)
        }

        #[test]
        fn two_member() {
            let input = r#"
                <value>
                    <struct>
                        <member>
                            <name>_1</name>
                            <value><int>1</int></value>
                        </member>
                        <member>
                            <name>_2</name>
                            <value><int>2</int></value>
                        </member>
                    </struct>
                </value>"#;

            let actual: HashMap<String, Value> = value_from_str(input).unwrap();

            let mut expected = HashMap::new();
            expected.insert("_1".to_string(), Value::Int(1));
            expected.insert("_2".to_string(), Value::Int(2));
            assert_eq!(actual, expected)
        }
    }

    mod ignore_any {
        use serde::de::IgnoredAny;

        use super::*;

        #[test]
        fn empty() {
            let input = r#"<value></value>"#;

            assert!(value_from_str::<IgnoredAny>(input).is_ok());
        }

        #[test]
        fn implicit_string() {
            let input = r#"<value>abc</value>"#;

            assert!(value_from_str::<IgnoredAny>(input).is_ok());
        }

        #[test]
        fn complex() {
            let input = r#"<value><struct>
                    <member>
                        <name>_1</name>
                        <value><int>1</int></value>
                    </member>
                    <member>
                        <name>_2</name>
                        <value><int>2</int></value>
                    </member>
                </struct></value>"#;

            assert!(value_from_str::<IgnoredAny>(input).is_ok());
        }
    }

    mod real_world_examples {
        use crate::MethodResponse;

        use super::*;

        #[test]
        fn method_list() {
            let input = r#"<?xml version="1.0" encoding="ISO-8859-1"?><methodResponse><params><param><value><array><data><value>getLinkPeers</value></data></array></value></param></params></methodResponse>"#;
            let actual: MethodResponse<Vec<String>> = method_response_from_str(input).unwrap();
            if let MethodResponse::Success(list) = actual {
                assert_eq!(vec!["getLinkPeers".to_string()], list)
            } else {
                assert!(false)
            }
        }
    }

    // TODO: tuple, vec, enum value
}
