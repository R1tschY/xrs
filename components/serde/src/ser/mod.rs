//! Module to handle custom serde `Serializer`

use std::io::Write;

use serde::ser::{self, Serialize};
use serde::serde_if_integer128;

use crate::Error;

use self::var::{Seq, Struct};
use crate::error::Reason;

mod attributes;
mod nonser;
mod var;

/// Serialize struct into a `Write`r
pub fn to_writer<W: Write, S: Serialize>(writer: W, value: &S) -> Result<(), Error> {
    let mut xml_writer = Writer::new(writer);
    let mut serializer = Serializer::new(&mut xml_writer);
    value.serialize(&mut serializer)
}

/// Serialize struct into a `String`
pub fn to_string<S: Serialize>(value: &S) -> Result<String, Error> {
    let mut writer = Vec::new();
    to_writer(&mut writer, value)?;
    let s = String::from_utf8(writer).map_err(|e| quick_xml::Error::Utf8(e.utf8_error()))?;
    Ok(s)
}

/// A Serializer
pub struct Serializer<'r, 'a, W: Write> {
    pub(crate) writer: &'a mut Writer<W>,
    /// Name of the root tag. If not specified, deduced from the structure name
    root_tag: Option<&'r str>,
}

impl<'r, 'a, W: Write> Serializer<'r, 'a, W> {
    /// Creates a new `Serializer` that uses struct name as a root tag name.
    ///
    /// Note, that attempt to serialize a non-struct (including unit structs
    /// and newtype structs) will end up to an error. Use `with_root` to create
    /// serializer with explicitly defined root element name
    pub fn new(writer: &'a mut Writer<W>) -> Self {
        Self::new_with_root(writer, None)
    }

    /// Creates a new `Serializer` that uses specified root tag name
    ///
    /// # Examples
    ///
    /// When serializing a primitive type, only its representation will be written:
    ///
    /// ```edition2018
    /// # use serde::Serialize;
    /// use serde_xml_adapt::{Writer, Serializer};
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::new_with_root(&mut writer, Some("root"));
    ///
    /// "node".serialize(&mut ser).unwrap();
    /// assert_eq!(String::from_utf8(buffer).unwrap(), "<root>node</root>");
    /// ```
    ///
    /// When serializing a struct, newtype struct, unit struct or tuple `root_tag`
    /// is used as tag name of root(s) element(s):
    ///
    /// ```edition2018
    /// # use serde::Serialize;
    /// use serde_xml_adapt::{Writer, Serializer};
    ///
    /// #[derive(Debug, PartialEq, Serialize)]
    /// struct Struct {
    ///     #[serde(rename = "@question")]
    ///     question: String,
    ///     #[serde(rename = "@answer")]
    ///     answer: u32,
    /// }
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::new_with_root(&mut writer, Some("root"));
    ///
    /// Struct {
    ///     question: "The Ultimate Question of Life, the Universe, and Everything".into(),
    ///     answer: 42,
    /// }.serialize(&mut ser).unwrap();
    /// assert_eq!(
    ///     String::from_utf8(buffer.clone()).unwrap(),
    ///     r#"<root question="The Ultimate Question of Life, the Universe, and Everything" answer="42"/>"#
    /// );
    /// ```
    pub fn new_with_root(writer: &'a mut Writer<W>, root_tag: Option<&'r str>) -> Self {
        Self { writer, root_tag }
    }

    fn write_primitive<P: std::fmt::Display>(
        &mut self,
        value: P,
        escaped: bool,
    ) -> Result<(), Error> {
        let value = value.to_string().into_bytes();
        let event = if escaped {
            BytesText::from_escaped(value)
        } else {
            BytesText::from_plain(&value)
        };

        self.render_tag_around(|writer| Ok(writer.write_event(Event::Text(event))?))
    }

    /// Writes self-closed tag `<tag_name/>` into inner writer
    fn write_self_closed(&mut self, tag_name: &str) -> Result<(), Error> {
        self.writer
            .write_event(Event::Empty(BytesStart::borrowed_name(tag_name.as_bytes())))?;
        Ok(())
    }

    fn render_tag_around(
        &mut self,
        f: impl FnOnce(&mut Writer<W>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if let Some(root) = self.root_tag {
            self.write_tag_start(root)?;
            f(self.writer)?;
            self.write_tag_end(root)
        } else {
            f(self.writer)
        }
    }

    fn write_tag_start(&mut self, tag: &str) -> Result<(), Error> {
        Ok(self
            .writer
            .write_event(Event::Start(BytesStart::borrowed_name(tag.as_bytes())))?)
    }

    fn write_tag_end(&mut self, tag: &str) -> Result<(), Error> {
        Ok(self
            .writer
            .write_event(Event::End(BytesEnd::borrowed(tag.as_bytes())))?)
    }

    fn error(&self, reason: Reason) -> Error {
        // TODO: set offset
        Error::new(reason, 0)
    }
}

impl<'r, 'a, 'w, W: Write> ser::Serializer for &'w mut Serializer<'r, 'a, W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Seq<'r, 'w, 'a, W>;
    type SerializeTuple = Seq<'r, 'w, 'a, W>;
    type SerializeTupleStruct = Seq<'r, 'w, 'a, W>;
    type SerializeTupleVariant = Seq<'r, 'w, 'a, W>;
    type SerializeMap = Struct<'r, 'w, 'a, W>;
    type SerializeStruct = Struct<'r, 'w, 'a, W>;
    type SerializeStructVariant = Struct<'r, 'w, 'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Error> {
        self.write_primitive(if v { "true" } else { "false" }, true)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    serde_if_integer128! {
        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Error> {
            self.write_primitive(v, true)
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Error> {
            self.write_primitive(v, true)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Error> {
        self.write_primitive(v, true)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Error> {
        self.write_primitive(v, false)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Error> {
        self.write_primitive(value, false)
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Error> {
        // TODO: I imagine you'd want to use base64 here.
        // Not sure how to roundtrip effectively though...
        Err(self.error(Reason::Unsupported("serialize_bytes")))
    }

    fn serialize_none(self) -> Result<Self::Ok, Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Error> {
        self.write_self_closed(self.root_tag.unwrap_or(name))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Error> {
        self.render_tag_around(|writer| {
            writer.write_event(Event::Empty(BytesStart::borrowed_name(variant.as_bytes())))?;
            Ok(())
        })
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error> {
        self.render_tag_around(|writer| {
            let mut serializer = Serializer::new_with_root(writer, Some(variant));
            value.serialize(&mut serializer)
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(Seq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Ok(Seq::new(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Ok(Seq::new(self))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        if let Some(root) = self.root_tag {
            self.write_tag_start(root)?;
        }

        Ok(Seq::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(Struct::new(self, self.root_tag.unwrap()))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Ok(Struct::new(self, self.root_tag.unwrap_or(name)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        if let Some(root) = self.root_tag {
            self.write_tag_start(root)?;
        }

        Ok(Struct::new(self, variant))
    }
}

#[cfg(test)]
mod tests {
    use serde::ser::SerializeMap;
    use serde::Serializer as SerSerializer;
    use serde_derive::Serialize;

    use super::*;

    pub fn to_string_with_root<S: Serialize>(value: &S, root_tag: &str) -> Result<String, Error> {
        let mut buffer = Vec::new();
        let mut xml_writer = Writer::new(&mut buffer);
        let mut serializer = Serializer::new_with_root(&mut xml_writer, Some(root_tag));
        value.serialize(&mut serializer)?;
        let s = String::from_utf8(buffer).map_err(|e| quick_xml::Error::Utf8(e.utf8_error()))?;
        Ok(s)
    }

    #[derive(Serialize)]
    struct Person {
        name: String,
        age: u32,
    }

    #[derive(Serialize)]
    struct PersonAttrs {
        #[serde(rename = "@name")]
        name: String,
        #[serde(rename = "@age")]
        age: u32,
    }

    #[test]
    fn test_serialize_bool() {
        let inputs = vec![(true, "true"), (false, "false")];

        for (src, should_be) in inputs {
            assert_eq!(to_string(&src).unwrap(), should_be);
        }
    }

    #[test]
    fn empty_string() {
        let value: Option<String> = Some(String::new());
        assert_eq!(
            to_string_with_root(&value, "root").unwrap(),
            "<root></root>"
        );
    }

    #[test]
    fn null_string() {
        let value: Option<String> = None;
        assert_eq!(to_string_with_root(&value, "root").unwrap(), "");
    }

    #[test]
    fn test_serialize_struct_attrs() {
        let bob = PersonAttrs {
            name: "Bob".to_string(),
            age: 42,
        };
        assert_eq!(
            to_string(&bob).unwrap(),
            "<PersonAttrs name=\"Bob\" age=\"42\"/>"
        );
    }

    #[test]
    fn test_serialize_struct() {
        let bob = Person {
            name: "Bob".to_string(),
            age: 42,
        };
        assert_eq!(
            to_string(&bob).unwrap(),
            "<Person><name>Bob</name><age>42</age></Person>"
        );
    }

    #[test]
    fn test_serialize_escaped_attrs() {
        let bob = PersonAttrs {
            name: "<?<!-- '\" -->".to_string(),
            age: 42,
        };
        assert_eq!(
            to_string(&bob).unwrap(),
            "<PersonAttrs name=\"&lt;?&lt;!-- &apos;&quot; --&gt;\" age=\"42\"/>"
        );
    }

    #[test]
    fn test_serialize_map_entries() {
        let mut buffer = Vec::new();
        let mut xml_writer = Writer::new(&mut buffer);
        let mut serializer = Serializer::new_with_root(&mut xml_writer, Some("root"));

        let mut map = serializer.serialize_map(Some(2)).unwrap();
        map.serialize_entry("name", "Bob").unwrap();
        map.serialize_entry("age", "5").unwrap();
        map.end().unwrap();

        let actual = String::from_utf8(buffer)
            .map_err(|e| quick_xml::Error::Utf8(e.utf8_error()))
            .unwrap();

        assert_eq!(actual, "<root><name>Bob</name><age>5</age></root>");
    }

    #[test]
    fn serialize_a_list() {
        let data = vec![1, 2, 3, 4];

        assert_eq!(
            to_string_with_root(&data, "root").unwrap(),
            "<root>1</root><root>2</root><root>3</root><root>4</root>"
        );
    }

    #[test]
    fn unit() {
        #[derive(Serialize)]
        struct Unit;

        assert_eq!(to_string_with_root(&Unit, "root").unwrap(), "<root/>");
    }

    #[test]
    fn named_tuple1() {
        #[derive(Serialize)]
        struct Tuple(bool);

        assert_eq!(
            to_string_with_root(&Tuple(true), "root").unwrap(),
            "<root>true</root>"
        );
    }

    #[test]
    fn named_tuple3() {
        #[derive(Serialize)]
        struct Tuple(bool, bool, bool);

        assert_eq!(
            to_string_with_root(&Tuple(true, false, true), "root").unwrap(),
            "<root>true</root><root>false</root><root>true</root>"
        );
    }

    #[test]
    fn tuple() {
        let data = (42.0, "answer");
        assert_eq!(
            to_string_with_root(&data, "root").unwrap(),
            "<root>42</root><root>answer</root>"
        );
    }

    #[test]
    fn tuple_struct() {
        #[derive(Serialize)]
        struct Tuple(f32, &'static str);

        let data = Tuple(42.0, "answer");
        assert_eq!(
            to_string_with_root(&data, "root").unwrap(),
            "<root>42</root><root>answer</root>"
        );
    }

    #[test]
    fn nested_struct() {
        #[derive(Serialize)]
        struct Struct {
            nested: Nested,
            string: String,
        }

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        let expected = r#"<root><nested><float>42</float></nested><string>answer</string></root>"#;
        let data = Struct {
            nested: Nested { float: 42.0 },
            string: "answer".to_string(),
        };
        assert_eq!(to_string_with_root(&data, "root").unwrap(), expected);
    }

    #[test]
    fn flatten_struct() {
        #[derive(Serialize)]
        struct Struct {
            #[serde(flatten)]
            nested: Nested,
            string: String,
        }

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        let expected = r#"<root><float>42</float><string>answer</string></root>"#;
        let data = Struct {
            nested: Nested { float: 42.0 },
            string: "answer".to_string(),
        };
        assert_eq!(to_string_with_root(&data, "root").unwrap(), expected);
    }

    mod enum_ {
        use super::*;

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        mod externally_tagged {
            use super::*;

            #[derive(Serialize)]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                assert_eq!(
                    to_string_with_root(&Node::Unit, "root").unwrap(),
                    "<root><Unit/></root>"
                );
            }

            #[test]
            fn newtype() {
                assert_eq!(
                    to_string_with_root(&Node::Newtype(true), "root").unwrap(),
                    "<root><Newtype>true</Newtype></root>"
                );
            }

            #[test]
            fn struct_() {
                let node = Node::Struct {
                    float: 42.0,
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><Struct><float>42</float><string>answer</string></Struct></root>"#
                );
            }

            #[test]
            #[ignore]
            fn tuple_struct() {
                let node = Node::Tuple(42.0, "answer".to_string());

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><Tuple>42</Tuple><Tuple>answer</Tuple></root>"#
                );
            }

            #[test]
            fn nested_struct() {
                let node = Node::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><Holder><nested><float>42</float></nested><string>answer</string></Holder></root>"#
                );
            }

            #[test]
            fn flatten_struct() {
                let node = Node::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><Flatten><float>42</float><string>answer</string></Flatten></root>"#
                );
            }
        }

        mod internally_tagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(tag = "@tag")]
            enum Node {
                Unit,
                /// Primitives (such as `bool`) are not supported by the serde in the internally tagged mode
                Newtype(NewtypeContent),
                // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[derive(Serialize)]
            struct NewtypeContent {
                value: bool,
            }

            #[test]
            fn unit() {
                assert_eq!(
                    to_string_with_root(&Node::Unit, "root").unwrap(),
                    r#"<root tag="Unit"/>"#
                );
            }

            #[test]
            fn newtype() {
                assert_eq!(
                    to_string_with_root(&Node::Newtype(NewtypeContent { value: true }), "root")
                        .unwrap(),
                    r#"<root tag="Newtype"><value>true</value></root>"#
                );
            }

            #[test]
            fn struct_() {
                let node = Node::Struct {
                    float: 42.0,
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root tag="Struct"><float>42</float><string>answer</string></root>"#
                );
            }

            #[test]
            fn nested_struct() {
                let node = Node::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root tag="Holder"><nested><float>42</float></nested><string>answer</string></root>"#
                );
            }

            #[test]
            fn flatten_struct() {
                let node = Node::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root tag="Flatten"><float>42</float><string>answer</string></root>"#
                );
            }
        }

        mod adjacently_tagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(tag = "@tag", content = "content")]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                assert_eq!(
                    to_string_with_root(&Node::Unit, "root").unwrap(),
                    r#"<root tag="Unit"/>"#
                );
            }

            #[test]
            fn newtype() {
                assert_eq!(
                    to_string_with_root(&Node::Newtype(true), "root").unwrap(),
                    r#"<root tag="Newtype"><content>true</content></root>"#
                );
            }

            #[test]
            fn tuple_struct() {
                assert_eq!(
                    to_string_with_root(&Node::Tuple(42.0, "answer".to_string()), "root").unwrap(),
                    r#"<root tag="Tuple"><content>42</content><content>answer</content></root>"#
                );
            }

            #[test]
            fn struct_() {
                assert_eq!(
                    to_string_with_root(
                        &Node::Struct {
                            float: 42.0,
                            string: "answer".to_string()
                        },
                        "root"
                    )
                    .unwrap(),
                    r#"<root tag="Struct"><content><float>42</float><string>answer</string></content></root>"#
                );
            }

            #[test]
            fn nested_struct() {
                let node = Node::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root tag="Holder"><content><nested><float>42</float></nested><string>answer</string></content></root>"#
                );
            }

            #[test]
            fn flatten_struct() {
                let node = Node::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root tag="Flatten"><content><float>42</float><string>answer</string></content></root>"#
                );
            }
        }

        mod untagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(untagged)]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                assert_eq!(to_string_with_root(&Node::Unit, "root").unwrap(), r#""#);
            }

            #[test]
            fn newtype() {
                assert_eq!(
                    to_string_with_root(&Node::Newtype(true), "root").unwrap(),
                    r#"<root>true</root>"#
                );
            }

            #[test]
            fn tuple_struct() {
                assert_eq!(
                    to_string_with_root(&Node::Tuple(42.0, "answer".to_string()), "root").unwrap(),
                    r#"<root>42</root><root>answer</root>"#
                );
            }

            #[test]
            fn struct_() {
                let node = Node::Struct {
                    float: 42.0,
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><float>42</float><string>answer</string></root>"#
                );
            }

            #[test]
            fn nested_struct() {
                let node = Node::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><nested><float>42</float></nested><string>answer</string></root>"#
                );
            }

            #[test]
            fn flatten_struct() {
                let node = Node::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><float>42</float><string>answer</string></root>"#
                );
            }
        }

        mod without_attrs {
            use super::*;

            #[test]
            fn internally_tagged() {
                #[derive(Serialize)]
                #[serde(tag = "tag")]
                enum InternallyTagged {
                    #[serde(rename = "flatten")]
                    Flatten {
                        #[serde(flatten)]
                        nested: Nested,
                        string: String,
                    },
                }

                let node = InternallyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><tag>flatten</tag><float>42</float><string>answer</string></root>"#
                );
            }

            #[test]
            fn adjacently_tagged() {
                #[derive(Serialize)]
                #[serde(tag = "tag", content = "content")]
                enum AdjacentlyTagged {
                    #[serde(rename = "flatten")]
                    Flatten {
                        #[serde(flatten)]
                        nested: Nested,
                        string: String,
                    },
                }

                let node = AdjacentlyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer".to_string(),
                };

                assert_eq!(
                    to_string_with_root(&node, "root").unwrap(),
                    r#"<root><tag>flatten</tag><content><float>42</float><string>answer</string></content></root>"#
                );
            }
        }

        #[test]
        fn value_enum() {
            #[derive(Serialize)]
            #[serde(rename_all = "lowercase", tag = "$value")]
            enum Enum {
                One,
                Two,
                Three,
            }

            assert_eq!(
                to_string_with_root(&[Enum::One, Enum::Two, Enum::Three], "root").unwrap(),
                r#"<root>one</root><root>two</root><root>three</root>"#
            );
        }

        #[test]
        fn value_enum_as_attr() {
            #[derive(Serialize)]
            #[serde(rename_all = "lowercase")]
            enum Enum {
                One,
            }

            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "@attr")]
                attr: Enum,
            }

            assert_eq!(
                to_string_with_root(&Node { attr: Enum::One }, "root").unwrap(),
                r#"<root attr="one"/>"#
            );
        }
    }

    mod inline_content {
        use super::*;

        #[test]
        fn string_content() {
            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "@attr")]
                attr: i32,
                #[serde(rename = "$")]
                content: i32,
            }

            let node = Node {
                attr: 42,
                content: 5,
            };
            assert_eq!(
                to_string_with_root(&node, "root").unwrap(),
                r#"<root attr="42">5</root>"#
            );
        }

        #[test]
        fn multiple_string_content() {
            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "@attr")]
                attr: i32,
                #[serde(rename = "$")]
                chars1: i32,
                tagged1: i32,
                #[serde(rename = "$")]
                chars2: i32,
                tagged2: i32,
                #[serde(rename = "$")]
                chars3: Vec<i32>,
            }

            let node = Node {
                attr: 42,
                chars1: 5,
                tagged1: 1,
                chars2: 5,
                tagged2: 1,
                chars3: vec![34, 45],
            };
            assert_eq!(
                to_string_with_root(&node, "root").unwrap(),
                r#"<root attr="42">5<tagged1>1</tagged1>5<tagged2>1</tagged2>3445</root>"#
            );
        }

        #[test]
        fn attributes() {
            #[derive(Serialize)]
            struct Nested {
                #[serde(rename = "@attr")]
                attr: i32,
            }

            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "$")]
                nested: Nested,
            }

            let node = Node {
                nested: Nested { attr: 42 },
            };
            assert_eq!(
                to_string_with_root(&node, "root").unwrap(),
                r#"<root><Nested attr="42"/></root>"#
            );
        }
    }

    mod optional {
        use super::*;

        #[derive(Serialize)]
        struct Node {
            #[serde(rename = "@attr")]
            attr: Option<i32>,
        }

        #[test]
        fn some() {
            assert_eq!(
                to_string_with_root(&Some(42), "root").unwrap(),
                r#"<root>42</root>"#
            );
        }

        #[test]
        fn none() {
            let x: Option<i32> = None;
            assert_eq!(to_string_with_root(&x, "root").unwrap(), r#""#);
        }

        #[test]
        fn attribute_some() {
            assert_eq!(
                to_string_with_root(&Node { attr: Some(42) }, "root").unwrap(),
                r#"<root attr="42"/>"#
            );
        }

        #[test]
        fn attribute_none() {
            assert_eq!(
                to_string_with_root(&Node { attr: None }, "root").unwrap(),
                r#"<root/>"#
            );
        }
    }

    mod namespaces {
        use super::*;

        #[test]
        fn attribute() {
            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "@xsi:type")]
                ty: String,
            }

            assert_eq!(
                to_string_with_root(
                    &Node {
                        ty: "string".to_string()
                    },
                    "root"
                )
                .unwrap(),
                r#"<root xsi:type="string"/>"#
            );
        }

        #[test]
        fn tag() {
            #[derive(Serialize)]
            struct Node {
                #[serde(rename = "xsi:element")]
                ty: String,
            }

            assert_eq!(
                to_string_with_root(
                    &Node {
                        ty: "string".to_string()
                    },
                    "root"
                )
                .unwrap(),
                r#"<root><xsi:element>string</xsi:element></root>"#
            );
        }
    }
}
