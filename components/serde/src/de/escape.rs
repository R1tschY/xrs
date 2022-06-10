//! Serde `Deserializer` module

use std::borrow::Cow;

use serde::de::{self, Visitor};
use serde::{self, forward_to_deserialize_any};

use crate::{error::Reason, error::ResultExt, Error, Result};

#[derive(Clone)]
pub(crate) struct EscapedDeserializer<'de> {
    value: Cow<'de, str>,
}

impl<'de> EscapedDeserializer<'de> {
    pub fn new(value: Cow<'de, str>) -> Self {
        Self { value }
    }

    fn error(&self, reason: Reason) -> Error {
        Error::new(reason, 0)
    }
}

macro_rules! deserialize_num {
    ($method:ident, $ty:path, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            let value = self.value.parse::<$ty>().at_offset(0)?;
            visitor.$visit(value)
        }
    };
}

impl<'de> serde::Deserializer<'de> for EscapedDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &*self.value {
            "true" | "1" => visitor.visit_bool(true),
            "false" | "0" => visitor.visit_bool(false),
            e => Err(self.error(Reason::InvalidBoolean(e.to_string()))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Cow::Borrowed(borrowed) => visitor.visit_borrowed_str(borrowed),
            Cow::Owned(owned) => visitor.visit_string(owned),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Cow::Borrowed(borrowed) => visitor.visit_borrowed_bytes(borrowed.as_bytes()),
            Cow::Owned(owned) => visitor.visit_byte_buf(owned.into_bytes()),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.value.is_empty() {
            visitor.visit_unit()
        } else {
            Err(self.error(Reason::InvalidUnit(
                "Expecting unit, got non empty attribute".into(),
            )))
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self)
    }

    deserialize_num!(deserialize_i64, i64, visit_i64);
    deserialize_num!(deserialize_i32, i32, visit_i32);
    deserialize_num!(deserialize_i16, i16, visit_i16);
    deserialize_num!(deserialize_i8, i8, visit_i8);
    deserialize_num!(deserialize_u64, u64, visit_u64);
    deserialize_num!(deserialize_u32, u32, visit_u32);
    deserialize_num!(deserialize_u16, u16, visit_u16);
    deserialize_num!(deserialize_u8, u8, visit_u8);
    deserialize_num!(deserialize_f64, f64, visit_f64);
    deserialize_num!(deserialize_f32, f32, visit_f32);

    forward_to_deserialize_any! {
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

impl<'de> de::EnumAccess<'de> for EscapedDeserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        let name = seed.deserialize(self.clone())?;
        Ok((name, self))
    }
}

impl<'de> de::VariantAccess<'de> for EscapedDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: de::Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        unimplemented!()
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value> {
        unimplemented!()
    }
}
