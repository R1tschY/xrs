use serde::de::value::CowStrDeserializer;
use serde::de::{self, Deserializer as SerdeDeserializer, IntoDeserializer};
use xrs_parser::XmlEvent;

use crate::de::Deserializer;
use crate::error::Reason;
use crate::Error;

/// An enum access
pub struct EnumAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> EnumAccess<'a, 'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = VariantAccess<'a, 'de>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'a, 'de>), Error> {
        if let XmlEvent::STag(e) = self.de.next()? {
            let de: CowStrDeserializer<'de, Error> = e.name.into_deserializer();
            let ident = seed.deserialize(de)?;
            Ok((ident, VariantAccess { de: self.de }))
        } else {
            dbg!("var");
            return Err(self.de.peek_error(Reason::Start));
        }
    }
}

pub struct VariantAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'de, 'a> de::VariantAccess<'de> for VariantAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Err(Error::new(Reason::NoMarkupExpected, 0))
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.de.deserialize_struct("", fields, visitor)
    }
}
