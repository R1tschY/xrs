use std::borrow::Cow;
use std::io::Write;

use serde::ser::{self, Serialize};
use xrs_parser::{Attribute, STag};

use crate::error::Reason;
use crate::ser::attributes::AttributeSerializer;
use crate::ser::Serializer;
use crate::Error;

/// An implementation of `SerializeStruct` for serializing to XML.
pub struct Struct<'r, 'w, 'a, W>
where
    W: Write,
{
    parent: &'w mut Serializer<'r, 'a, W>,
    /// Buffer for holding fields, serialized as attributes. Doesn't allocate
    /// if there are no fields represented as attributes
    attrs: Vec<Attribute<'r>>,
    /// Buffer for holding fields, serialized as elements
    children: Vec<u8>,
    /// Buffer for serializing one field. Cleared after serialize each field
    buffer: Vec<u8>,
}

impl<'r, 'a, 'w, W> Struct<'r, 'w, 'a, W>
where
    W: 'w + Write,
{
    /// Create a new `Struct`
    pub fn new(parent: &'w mut Serializer<'r, 'a, W>, name: &'w str) -> Self {
        Struct {
            parent,
            attrs: Vec::new(),
            children: Vec::new(),
            buffer: Vec::new(),
        }
    }

    fn serialize_tag<T: ?Sized + Serialize>(
        &mut self,
        key: Cow<'static, &str>,
        value: &T,
    ) -> Result<(), Error> {
        // TODO: Inherit indentation state from self.parent.writer

        if key.starts_with("@") {
            if key.len() == 1 {
                return Err(self
                    .parent
                    .error(Reason::Message("name for attribute is missing".to_string())));
            }

            let mut serializer = AttributeSerializer::new();
            let attribute_value = value.serialize(&mut serializer)?;
            if let Some(attribute_value) = attribute_value {
                self.attrs.push(Attribute {
                    name: key.tail(1),
                    value: attribute_value.into(),
                });
            }
            self.buffer.clear();
        } else {
            let root = if key.starts_with("$") {
                None
            } else {
                Some(key)
            };
            let mut writer = Writer::new(&mut self.buffer);
            let mut serializer = Serializer::new_with_root(&mut writer, root.map(|s| s.as_ref()));
            value.serialize(&mut serializer)?;

            self.children.append(&mut self.buffer);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<(), Error> {
        let writer = &mut self.parent.writer;
        if self.children.is_empty() {
            writer.write_event(Event::Empty(self.attrs.to_borrowed()))?;
        } else {
            writer.write_event(Event::Start(self.attrs.to_borrowed()))?;
            writer.write(&self.children)?;
            writer.write_event(Event::End(self.attrs.to_end()))?;
        }
        Ok(())
    }
}

impl<'r, 'w, 'a, W> ser::SerializeStruct for Struct<'r, 'w, 'a, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.serialize_tag(key, value)
    }

    fn end(mut self) -> Result<Self::Ok, Error> {
        self.close()
    }
}

impl<'r, 'w, 'a, W> ser::SerializeStructVariant for Struct<'r, 'w, 'a, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.serialize_tag(key, value)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.close()?;

        if let Some(root) = self.parent.root_tag {
            self.parent.write_tag_end(root)
        } else {
            Ok(())
        }
    }
}

impl<'r, 'a, 'w, W> ser::SerializeMap for Struct<'r, 'a, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Err(self.parent.error(Reason::Unsupported(
            "impossible to serialize the key on its own, please use serialize_entry()",
        )))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.parent)
    }

    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Error> {
        // TODO: use own TagSerializer
        let tag = key.serialize(&mut AttributeSerializer::new())?;
        if let Some(tag) = tag {
            self.serialize_tag(&tag, value)
        } else {
            Err(self.parent.error(Reason::Message(
                "Option as map key not supported".to_string(),
            )))
        }
    }

    fn end(mut self) -> Result<Self::Ok, Error> {
        self.close()
    }
}

/// An implementation of `SerializeSeq`, `SerializeTuple`, `SerializeTupleStruct` and
/// `SerializeTupleVariant` for serializing to XML.
pub struct Seq<'r, 'w, 'a, W>
where
    W: Write,
{
    parent: &'w mut Serializer<'r, 'a, W>,
}

impl<'r, 'w, 'a, W> Seq<'r, 'w, 'a, W>
where
    W: Write,
{
    /// Create a new `Tuple`
    pub fn new(parent: &'w mut Serializer<'r, 'a, W>) -> Self {
        Seq { parent }
    }

    fn serialize_item<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.parent)
    }
}

impl<'r, 'w, 'a, W> ser::SerializeSeq for Seq<'r, 'w, 'a, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_item(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'r, 'a, 'w, W> ser::SerializeTuple for Seq<'r, 'a, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_item(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'r, 'a, 'w, W> ser::SerializeTupleStruct for Seq<'r, 'a, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_item(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'r, 'a, 'w, W> ser::SerializeTupleVariant for Seq<'r, 'a, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_item(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        if let Some(root) = self.parent.root_tag {
            self.parent.write_tag_end(root)
        } else {
            Ok(())
        }
    }
}
