//! Serde `Deserializer` module

use std::borrow::Cow;

use serde::de::{self, DeserializeSeed, IntoDeserializer};

use crate::{
    de::{escape::EscapedDeserializer, Deserializer, INNER_VALUE},
    Error,
};
use xrs_parser::Attribute;

enum MapValue<'de> {
    Empty,
    Attribute { value: Cow<'de, str> },
    Nested,
    InnerValue,
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    attributes: std::vec::IntoIter<Attribute<'de>>,
    value: MapValue<'de>,
    has_value_field: bool,
}

impl<'a, 'de> MapAccess<'a, 'de> {
    fn create_attr_key(key: &str) -> String {
        // TODO: optimize copies!
        let mut result = String::with_capacity(key.len() + 1);
        result.push('@');
        result.push_str(key);
        result
    }

    /// Create a new MapAccess
    pub fn new(de: &'a mut Deserializer<'de>, has_value_field: bool) -> Result<Self, Error> {
        let attributes = de.reader.drain_attributes().into_iter();
        Ok(MapAccess {
            de,
            attributes,
            value: MapValue::Empty,
            has_value_field,
        })
    }
}

impl<'a, 'de> de::MapAccess<'de> for MapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if let Some(attr) = self.attributes.next() {
            // try getting map from attributes (key= "value")
            self.value = MapValue::Attribute { value: attr.value };
            seed.deserialize(Self::create_attr_key(&attr.name).into_deserializer())
                .map(Some)
        } else if self.has_value_field {
            self.value = MapValue::InnerValue;
            seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
        } else {
            self.value = MapValue::Nested;
            if let Some(stag) = self.de.next_maybe_start()? {
                seed.deserialize(stag.name.into_deserializer()).map(Some)
            } else {
                Ok(None)
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        match std::mem::replace(&mut self.value, MapValue::Empty) {
            MapValue::Attribute { value } => seed.deserialize(EscapedDeserializer::new(value)),
            MapValue::Nested | MapValue::InnerValue => seed.deserialize(&mut *self.de),
            MapValue::Empty => unreachable!(),
        }
    }
}
