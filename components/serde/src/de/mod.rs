//! Serde `Deserializer` module
//!
//! # Examples
//!
//! Here is a simple example parsing [crates.io](https://crates.io/) source code.
//!
//! ```
//! // Cargo.toml
//! // [dependencies]
//! // serde = { version = "1.0", features = [ "derive" ] }
//! // quick_xml = { version = "0.17", features = [ "serialize" ] }
//! extern crate serde;
//! extern crate xrs_serde;
//!
//! use serde::Deserialize;
//! use serde_explicit_xml::{from_str, Error};
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Link {
//!     rel: String,
//!     href: String,
//!     sizes: Option<String>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! #[serde(rename_all = "lowercase")]
//! enum Lang {
//!     En,
//!     Fr,
//!     De,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Head {
//!     title: String,
//!     #[serde(rename = "link", default)]
//!     links: Vec<Link>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Script {
//!     src: String,
//!     integrity: String,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Body {
//!     #[serde(rename = "script", default)]
//!     scripts: Vec<Script>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Html {
//!     lang: Option<String>,
//!     head: Head,
//!     body: Body,
//! }
//!
//! fn crates_io() -> Result<Html, Error> {
//!     let xml = "<!DOCTYPE html>
//!         <html lang=\"en\">
//!           <head>
//!             <meta charset=\"utf-8\">
//!             <meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">
//!             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
//!
//!             <title>crates.io: Rust Package Registry</title>
//!
//!
//!         <meta name=\"cargo/config/environment\" content=\"%7B%22modulePrefix%22%3A%22cargo%22%2C%22environment%22%3A%22production%22%2C%22rootURL%22%3A%22%2F%22%2C%22locationType%22%3A%22router-scroll%22%2C%22historySupportMiddleware%22%3Atrue%2C%22EmberENV%22%3A%7B%22FEATURES%22%3A%7B%7D%2C%22EXTEND_PROTOTYPES%22%3A%7B%22Date%22%3Afalse%7D%7D%2C%22APP%22%3A%7B%22name%22%3A%22cargo%22%2C%22version%22%3A%22b7796c9%22%7D%2C%22fastboot%22%3A%7B%22hostWhitelist%22%3A%5B%22crates.io%22%2C%7B%7D%2C%7B%7D%5D%7D%2C%22ember-cli-app-version%22%3A%7B%22version%22%3A%22b7796c9%22%7D%2C%22ember-cli-mirage%22%3A%7B%22usingProxy%22%3Afalse%2C%22useDefaultPassthroughs%22%3Atrue%7D%2C%22exportApplicationGlobal%22%3Afalse%7D\" />
//!         <!-- EMBER_CLI_FASTBOOT_TITLE --><!-- EMBER_CLI_FASTBOOT_HEAD -->
//!         <link rel=\"manifest\" href=\"/manifest.webmanifest\">
//!         <link rel=\"apple-touch-icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" sizes=\"227x227\">
//!         <meta name=\"theme-color\" content=\"#f9f7ec\">
//!         <meta name=\"apple-mobile-web-app-capable\" content=\"yes\">
//!         <meta name=\"apple-mobile-web-app-title\" content=\"crates.io: Rust Package Registry\">
//!         <meta name=\"apple-mobile-web-app-status-bar-style\" content=\"default\">
//!
//!             <link rel=\"stylesheet\" href=\"/assets/vendor-8d023d47762d5431764f589a6012123e.css\" integrity=\"sha256-EoB7fsYkdS7BZba47+C/9D7yxwPZojsE4pO7RIuUXdE= sha512-/SzGQGR0yj5AG6YPehZB3b6MjpnuNCTOGREQTStETobVRrpYPZKneJwcL/14B8ufcvobJGFDvnTKdcDDxbh6/A==\" >
//!             <link rel=\"stylesheet\" href=\"/assets/cargo-cedb8082b232ce89dd449d869fb54b98.css\" integrity=\"sha256-S9K9jZr6nSyYicYad3JdiTKrvsstXZrvYqmLUX9i3tc= sha512-CDGjy3xeyiqBgUMa+GelihW394pqAARXwsU+HIiOotlnp1sLBVgO6v2ZszL0arwKU8CpvL9wHyLYBIdfX92YbQ==\" >
//!
//!
//!             <link rel=\"shortcut icon\" href=\"/favicon.ico\" type=\"image/x-icon\">
//!             <link rel=\"icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" type=\"image/png\">
//!             <link rel=\"search\" href=\"/opensearch.xml\" type=\"application/opensearchdescription+xml\" title=\"Cargo\">
//!           </head>
//!           <body>
//!             <!-- EMBER_CLI_FASTBOOT_BODY -->
//!             <noscript>
//!                 <div id=\"main\">
//!                     <div class='noscript'>
//!                         This site requires JavaScript to be enabled.
//!                     </div>
//!                 </div>
//!             </noscript>
//!
//!             <script src=\"/assets/vendor-bfe89101b20262535de5a5ccdc276965.js\" integrity=\"sha256-U12Xuwhz1bhJXWyFW/hRr+Wa8B6FFDheTowik5VLkbw= sha512-J/cUUuUN55TrdG8P6Zk3/slI0nTgzYb8pOQlrXfaLgzr9aEumr9D1EzmFyLy1nrhaDGpRN1T8EQrU21Jl81pJQ==\" ></script>
//!             <script src=\"/assets/cargo-4023b68501b7b3e17b2bb31f50f5eeea.js\" integrity=\"sha256-9atimKc1KC6HMJF/B07lP3Cjtgr2tmET8Vau0Re5mVI= sha512-XJyBDQU4wtA1aPyPXaFzTE5Wh/mYJwkKHqZ/Fn4p/ezgdKzSCFu6FYn81raBCnCBNsihfhrkb88uF6H5VraHMA==\" ></script>
//!
//!
//!           </body>
//!         </html>
//! }";
//!     let html: Html = from_str(xml)?;
//!     assert_eq!(&html.head.title, "crates.io: Rust Package Registr");
//!     Ok(html)
//! }
//! ```

use std::borrow::Cow;
use std::io::BufRead;

use serde::de::{self, DeserializeOwned, IntoDeserializer, Visitor};
use serde::{serde_if_integer128, Deserialize};

use xrs_chars::XmlAsciiChar;
use xrs_parser::{Reader, STag, XmlEvent};

use crate::de::cow::{CowStrExt, StrExt};
use crate::error::Reason;
use crate::error::ResultExt;
use crate::Error;

mod cow;
mod escape;
mod map;
mod seq;
mod var;

const INNER_VALUE: &str = "$value";

/// An xml deserializer
pub struct Deserializer<'a> {
    reader: Reader<'a>,
    peek: Option<XmlEvent<'a>>,
}

/// Deserialize a xml string
pub fn from_str<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::new(Reader::new(s));
    T::deserialize(&mut de)
}

/// Deserialize from a reader
pub fn from_reader<R: BufRead, T: DeserializeOwned>(mut reader: R) -> Result<T, Error> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str(&buf)
}

impl<'a> Deserializer<'a> {
    /// Get a new deserializer
    pub fn new(reader: Reader<'a>) -> RootDeserializer<'a> {
        RootDeserializer {
            de: Self { reader, peek: None },
        }
    }

    /// Get a new deserializer from a regular BufRead
    pub fn from_str(data: &'a str) -> RootDeserializer<'a> {
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

        loop {
            if let Some(evt) = self.reader.next()? {
                match evt {
                    e @ (XmlEvent::STag(_) | XmlEvent::ETag(_) | XmlEvent::Characters(_)) => {
                        return Ok(e);
                    }
                    _ => (),
                }
            } else {
                return Err(Error::new(Reason::Eof, 0));
            }
        }
    }

    fn next_maybe_start(&mut self) -> Result<Option<STag<'a>>, Error> {
        loop {
            match self.next()? {
                XmlEvent::STag(e) => return Ok(Some(e)),
                XmlEvent::ETag(_) => {
                    return Ok(None);
                }
                XmlEvent::Characters(s) if !s.as_ref().is_xml_whitespace() => {
                    return Err(self.error(Reason::MarkupExpected))
                }
                _ => {}
            }
        }
    }

    /// Consumes Characters with terminating end tag
    fn next_text(&mut self) -> Result<Cow<'a, str>, Error> {
        let mut result = Cow::from(String::new());
        loop {
            match self.next()? {
                XmlEvent::Characters(chars) => {
                    if result.is_empty() {
                        result = chars;
                    } else {
                        result.push_str(&chars);
                    }
                }
                XmlEvent::ETag(_) => return Ok(result),
                _ => return Err(Error::new(Reason::NoMarkupExpected, 0)),
            }
        }
    }

    fn next_trimmed_text(&mut self) -> Result<Cow<'a, str>, Error> {
        let mut text = self.next_text()?;
        text.trim_matches(|c: char| c.is_xml_whitespace());
        Ok(text)
    }

    fn read_to_end(&mut self) -> Result<(), Error> {
        let mut depth = 1;
        while depth >= 0 {
            match self.next()? {
                XmlEvent::STag(_) => depth += 1,
                XmlEvent::ETag(_) => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    fn expect_end(&mut self) -> Result<(), Error> {
        match self.next()? {
            XmlEvent::ETag(_) => Ok(()),
            _ => Err(self.error(Reason::End)),
        }
    }

    fn skip_ignorable_and_whitespace(&mut self) -> Result<(), Error> {
        loop {
            match self.peek()? {
                XmlEvent::Characters(text) if text.as_ref().is_xml_whitespace() => {
                    self.next()?;
                }
                XmlEvent::Comment(_) | XmlEvent::PI(_) => (),
                _ => return Ok(()),
            }
        }
    }

    pub(crate) fn error(&self, reason: Reason) -> Error {
        Error::new(reason, self.reader.cursor_offset())
    }

    pub(crate) fn peek_error(&self, reason: Reason) -> Error {
        Error::new(reason, self.reader.cursor_offset())
    }

    pub(crate) fn fix_position(&self, err: Error) -> Error {
        err.fix_position(|reason| self.error(reason))
    }
}

macro_rules! deserialize_type {
    ($deserialize:ident => $ty:path, $visit:ident) => {
        fn $deserialize<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let value = self.next_trimmed_text()?.parse::<$ty>().at_offset(0)?;
            visitor.$visit(value)
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.skip_ignorable_and_whitespace()?;

        match self.peek()? {
            XmlEvent::STag(_) => self.deserialize_map(visitor),
            XmlEvent::ETag(_) => self.deserialize_unit(visitor),
            _ => self.deserialize_string(visitor),
        }
    }

    deserialize_type!(deserialize_i8 => i8, visit_i8);
    deserialize_type!(deserialize_i16 => i16, visit_i16);
    deserialize_type!(deserialize_i32 => i32, visit_i32);
    deserialize_type!(deserialize_i64 => i64, visit_i64);
    deserialize_type!(deserialize_u8 => u8, visit_u8);
    deserialize_type!(deserialize_u16 => u16, visit_u16);
    deserialize_type!(deserialize_u32 => u32, visit_u32);
    deserialize_type!(deserialize_u64 => u64, visit_u64);
    deserialize_type!(deserialize_f32 => f32, visit_f32);
    deserialize_type!(deserialize_f64 => f64, visit_f64);

    serde_if_integer128! {
        deserialize_type!(deserialize_i128 => i128, visit_i128);
        deserialize_type!(deserialize_u128 => u128, visit_u128);
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next_text()?.as_ref() {
            "true" | "1" | "yes" | "on" => visitor.visit_bool(true),
            "false" | "0" | "no" | "off" => visitor.visit_bool(false),
            e => Err(self.error(Reason::InvalidBoolean(e.to_string()))),
        }
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next_text()? {
            Cow::Borrowed(borrowed) => visitor.visit_str(borrowed),
            Cow::Owned(owned) => visitor.visit_string(owned),
        }
        .map_err(|err| self.fix_position(err))
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Error> {
        // TODO: use base64 or hex?
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next()? {
            XmlEvent::STag(_) => {
                self.expect_end()?;
                visitor.visit_unit().map_err(|err| self.fix_position(err))
            }
            e => Err(self.error(Reason::InvalidUnit(format!("{:?}", e)))),
        }
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
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor
            .visit_seq(seq::SeqAccess::new(self, None)?)
            .map_err(|err| self.fix_position(err))
    }

    fn deserialize_tuple<V: de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor
            .visit_seq(seq::SeqAccess::new(self, Some(len))?)
            .map_err(|err| self.fix_position(err))
    }

    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_struct("", &[], visitor)
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let map = map::MapAccess::new(self, fields.contains(&INNER_VALUE))?;
        let value = visitor
            .visit_map(map)
            .map_err(|err| self.fix_position(err))?;
        Ok(value)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.skip_ignorable_and_whitespace()?;

        match self.peek()? {
            XmlEvent::Characters(_) => visitor
                .visit_enum(self.next_trimmed_text()?.into_deserializer())
                .map_err(|err| self.fix_position(err)),
            _ => visitor
                .visit_enum(var::EnumAccess::new(self))
                .map_err(|err| self.fix_position(err)),
        }
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.next()? {
            XmlEvent::STag(_) => self.read_to_end()?,
            XmlEvent::ETag(_) => return Err(self.error(Reason::End)),
            _ => (),
        }
        visitor.visit_unit().map_err(|err| self.fix_position(err))
    }
}

pub struct RootDeserializer<'de> {
    de: Deserializer<'de>,
}

impl<'de> RootDeserializer<'de> {
    fn root_struct_error(&self) -> Error {
        self.de.error(Reason::RootStruct)
    }
}

macro_rules! forward_to_root_struct_error {
    ($($deserialize:ident)*) => {
        $(
            fn $deserialize<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
                Err(self.root_struct_error())
            }
        )*
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut RootDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Error> {
        Err(self.root_struct_error())
    }

    forward_to_root_struct_error!(
        deserialize_bool deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64
        deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64 deserialize_f32
        deserialize_f64 deserialize_char deserialize_str deserialize_string deserialize_bytes
        deserialize_byte_buf deserialize_option deserialize_unit deserialize_seq
        deserialize_identifier
    );

    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.de.next()? {
            XmlEvent::STag(e) if &e.name == name => {
                (&mut self.de).deserialize_unit_struct(name, visitor)
            }
            XmlEvent::STag(_) => Err(self.de.error(Reason::Tag(name))),
            _ => {
                dbg!("var");
                Err(self.de.error(Reason::Start))
            }
        }
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.de.next()? {
            XmlEvent::STag(e) if &e.name == name => {
                (&mut self.de).deserialize_newtype_struct(name, visitor)
            }
            XmlEvent::STag(_) => Err(self.de.error(Reason::Tag(name))),
            _ => {
                dbg!("var");
                Err(self.de.error(Reason::Start))
            }
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.root_struct_error())
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.root_struct_error())
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.root_struct_error())
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.de.next()? {
            XmlEvent::STag(e) if &e.name == name => {
                (&mut self.de).deserialize_struct(name, fields, visitor)
            }
            XmlEvent::STag(_) => Err(self.de.error(Reason::Tag(name))),
            _ => {
                dbg!("var");
                Err(self.de.error(Reason::Start))
            }
        }
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.de.next()? {
            XmlEvent::STag(e) if &e.name == name => {
                (&mut self.de).deserialize_enum(name, variants, visitor)
            }
            XmlEvent::STag(_) => Err(self.de.error(Reason::Tag(name))),
            _ => {
                dbg!("var");
                Err(self.de.error(Reason::Start))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[test]
    fn simple_struct_from_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename = "item")]
        struct Item {
            #[serde(rename = "@name")]
            name: String,
            #[serde(rename = "@source")]
            source: String,
        }

        let s = r##"
	        <item name="hello" source="world.rs" />
	    "##;

        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                name: "hello".to_string(),
                source: "world.rs".to_string(),
            }
        );
    }

    #[test]
    fn simple_struct_from_attribute_and_child() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "@name")]
            name: String,
            source: String,
        }

        let s = r##"
	        <item name="hello">
                <source>world.rs</source>
            </item>
        "##;

        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                name: "hello".to_string(),
                source: "world.rs".to_string(),
            }
        );
    }

    #[test]
    fn simple_struct_from_elements() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            name: String,
            source: String,
        }

        let s = r##"
	        <item>
	            <name>hello</name>
                <source>world.rs</source>
            </item>
        "##;

        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                name: "hello".to_string(),
                source: "world.rs".to_string(),
            }
        );
    }

    #[test]
    fn nested_collection() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename = "item")]
        struct Item {
            #[serde(rename = "@name")]
            name: String,
            #[serde(rename = "@source")]
            source: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Project {
            #[serde(rename = "@name")]
            name: String,

            #[serde(rename = "item", default)]
            items: Vec<Item>,
        }

        let s = r##"
	    <project name="my_project">
		<item name="hello1" source="world1.rs" />
		<item name="hello2" source="world2.rs" />
	    </project>
	"##;

        let project: Project = from_str(s).unwrap();

        assert_eq!(
            project,
            Project {
                name: "my_project".to_string(),
                items: vec![
                    Item {
                        name: "hello1".to_string(),
                        source: "world1.rs".to_string(),
                    },
                    Item {
                        name: "hello2".to_string(),
                        source: "world2.rs".to_string(),
                    },
                ],
            }
        );
    }

    #[test]
    fn collection_of_enums() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum MyEnum {
            A(String),
            B {
                #[serde(rename = "@name")]
                name: String,
                #[serde(rename = "@flag")]
                flag: bool,
            },
            C,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct MyEnums {
            // TODO: This should be #[serde(flatten)], but right now serde don't support flattening of sequences
            // See https://github.com/serde-rs/serde/issues/1905
            #[serde(rename = "$value")]
            items: Vec<MyEnum>,
        }

        let s = r##"
        <enums>
            <A>test</A>
            <B name="hello" flag="true" />
            <C />
        </enums>
        "##;

        let project: MyEnums = from_str(s).unwrap();

        assert_eq!(
            project,
            MyEnums {
                items: vec![
                    MyEnum::A("test".to_string()),
                    MyEnum::B {
                        name: "hello".to_string(),
                        flag: true,
                    },
                    MyEnum::C,
                ],
            }
        );
    }

    #[test]
    fn deserialize_bytes() {
        use serde::Deserialize;

        #[derive(Debug, PartialEq)]
        struct Item {
            bytes: Vec<u8>,
        }

        impl<'de> Deserialize<'de> for Item {
            fn deserialize<D>(d: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct ItemVisitor;

                impl<'de> de::Visitor<'de> for ItemVisitor {
                    type Value = Item;

                    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                        fmt.write_str("byte data")
                    }

                    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        Ok(Item { bytes: v.to_vec() })
                    }
                }

                Ok(d.deserialize_byte_buf(ItemVisitor)?)
            }
        }

        let s = r#"<item>bytes</item>"#;
        let item: Item = from_reader(s.as_bytes()).unwrap();

        assert_eq!(
            item,
            Item {
                bytes: "bytes".as_bytes().to_vec(),
            }
        );
    }

    /// Test for https://github.com/tafia/quick-xml/issues/231
    #[test]
    fn implicit_value() {
        use serde_value::Value;

        let s = r#"<root>content</root>"#;
        let item: Value = from_str(s).unwrap();

        assert_eq!(
            item,
            Value::Map(
                vec![(
                    Value::String("$value".into()),
                    Value::String("content".into())
                )]
                .into_iter()
                .collect()
            )
        );
    }

    #[test]
    fn explicit_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "$value")]
            content: String,
        }

        let s = r#"<root>content</root>"#;
        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                content: "content".into()
            }
        );
    }

    #[test]
    fn without_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item;

        let s = r#"<root>content</root>"#;
        let item: Item = from_str(s).unwrap();

        assert_eq!(item, Item);
    }

    #[test]
    fn unit() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Unit;

        let data: Unit = from_str("<root/>").unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn newtype() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Newtype(bool);

        let data: Newtype = from_str("<root>true</root>").unwrap();
        assert_eq!(data, Newtype(true));
    }

    #[test]
    fn tuple() {
        let data: (f32, String) = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }

    #[test]
    fn tuple_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Tuple(f32, String);

        let data: Tuple = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
    }

    mod struct_ {
        use super::*;

        #[test]
        fn elements() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                float: f64,
                string: String,
            }

            let data: Struct =
                from_str(r#"<root><float>42</float><string>answer</string></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                #[serde(rename = "@float")]
                float: f64,
                #[serde(rename = "@string")]
                string: String,
            }

            let data: Struct = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }
    }

    mod nested_struct {
        use super::*;

        #[test]
        fn elements() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                nested: Nested,
                string: String,
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                float: f32,
            }

            let data: Struct = from_str(
                r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
            )
            .unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: 42.0 },
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                nested: Nested,
                #[serde(rename = "@string")]
                string: String,
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                #[serde(rename = "@float")]
                float: f32,
            }

            let data: Struct =
                from_str(r#"<root string="answer"><nested float="42"/></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: 42.0 },
                    string: "answer".into()
                }
            );
        }
    }

    mod flatten_struct {
        use super::*;

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn elements() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
            }

            let data: Struct =
                from_str(r#"<root><float>42</float><string>answer</string></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: "42".into() },
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            #[derive(Debug, Deserialize, PartialEq)]
            struct Struct {
                #[serde(flatten)]
                nested: Nested,
                #[serde(rename = "@string")]
                string: String,
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                #[serde(rename = "@float")]
                float: String,
            }

            let data: Struct = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: "42".into() },
                    string: "answer".into()
                }
            );
        }
    }

    mod enum_ {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct NestedAttrs {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            #[serde(rename = "@float")]
            float: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct NewtypeContent {
            value: bool,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct NewtypeContentAttrs {
            #[serde(rename = "@value")]
            value: bool,
        }

        mod externally_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            enum Node {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
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

            #[derive(Debug, Deserialize, PartialEq)]
            enum NodeAttrs {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    #[serde(rename = "@float")]
                    float: f64,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Holder {
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            enum Workaround {
                Tuple(f64, String),
            }

            #[test]
            fn unit() {
                let data: Node = from_str("<Unit/>").unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn newtype() {
                let data: Node = from_str("<Newtype>true</Newtype>").unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            fn tuple_struct() {
                let data: Workaround = from_str("<Tuple>42</Tuple><Tuple>answer</Tuple>").unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            mod struct_ {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node =
                        from_str(r#"<Struct><float>42</float><string>answer</string></Struct>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<Struct float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<Holder><string>answer</string><nested><float>42</float></nested></Holder>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<Holder string="answer"><nested float="42"/></Holder>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Holder {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<Flatten><float>42</float><string>answer</string></Flatten>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<Flatten float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Flatten {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod internally_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag")]
            enum Node {
                Unit,
                /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
                Newtype(NewtypeContent),
                // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                Struct {
                    #[serde(rename = "@float")]
                    float: String,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Holder {
                    nested: Nested,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    #[serde(rename = "@string")]
                    string: String,
                },
            }

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "@tag")]
            enum NodeAttrs {
                Unit,
                /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
                Newtype(NewtypeContentAttrs),
                // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                Struct {
                    #[serde(rename = "@float")]
                    float: String,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Holder {
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
            }

            mod unit {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(r#"<root tag="Unit"/>"#).unwrap();
                    assert_eq!(data, NodeAttrs::Unit);
                }
            }

            mod newtype {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><tag>Newtype</tag><value>true</value></root>"#).unwrap();
                    assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root tag="Newtype" value="true"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Newtype(NewtypeContentAttrs { value: true })
                    );
                }
            }

            mod struct_ {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Struct</tag><float>42</float><string>answer</string></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: "42".into(),
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root tag="Struct" float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Struct {
                            float: "42".into(),
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Holder</tag><string>answer</string><nested><float>42</float></nested></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(
                        r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Holder {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root tag="Flatten" float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Flatten {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod adjacently_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag", content = "content")]
            enum Node {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
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

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "@tag", content = "content")]
            enum NodeAttrs {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    #[serde(rename = "@float")]
                    float: f64,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Holder {
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag", content = "content")]
            enum Workaround {
                Tuple(f64, String),
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "@tag", content = "content")]
            enum WorkaroundAttrs {
                Tuple(f64, String),
            }

            mod unit {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(r#"<root tag="Unit"/>"#).unwrap();
                    assert_eq!(data, NodeAttrs::Unit);
                }
            }

            mod newtype {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><tag>Newtype</tag><content>true</content></root>"#)
                            .unwrap();
                    assert_eq!(data, Node::Newtype(true));
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root tag="Newtype"><content>true</content></root>"#).unwrap();
                    assert_eq!(data, NodeAttrs::Newtype(true));
                }
            }

            mod tuple_struct {
                use super::*;

                #[test]
                fn elements() {
                    let data: Workaround = from_str(
                        r#"<root><tag>Tuple</tag><content>42</content><content>answer</content></root>"#
                    ).unwrap();
                    assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: WorkaroundAttrs = from_str(
                        r#"<root tag="Tuple"><content>42</content><content>answer</content></root>"#,
                    )
                    .unwrap();
                    assert_eq!(data, WorkaroundAttrs::Tuple(42.0, "answer".into()));
                }
            }

            mod struct_ {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Struct</tag><content><float>42</float><string>answer</string></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(
                        r#"<root tag="Struct"><content float="42" string="answer"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root>
                            <tag>Holder</tag>
                            <content>
                                <string>answer</string>
                                <nested>
                                    <float>42</float>
                                </nested>
                            </content>
                        </root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(
                        r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Holder {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Flatten</tag><content><float>42</float><string>answer</string></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs = from_str(
                        r#"<root tag="Flatten"><content float="42" string="answer"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Flatten {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod untagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(untagged)]
            enum Node {
                Unit,
                Newtype(bool),
                // serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
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
                    // Can't use "string" as name because in that case this variant
                    // will have no difference from `Struct` variant
                    string2: String,
                },
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(untagged)]
            enum Workaround {
                Tuple(f64, String),
            }

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(untagged)]
            enum NodeAttrs {
                Unit,
                Newtype(bool),
                // serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    #[serde(rename = "@float")]
                    float: f64,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Holder {
                    nested: NestedAttrs,
                    #[serde(rename = "@string")]
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: NestedAttrs,
                    // Can't use "string" as name because in that case this variant
                    // will have no difference from `Struct` variant
                    #[serde(rename = "@string2")]
                    string2: String,
                },
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn unit() {
                // Unit variant consists just from the tag, and because tags
                // are not written, nothing is written
                let data: Node = from_str("").unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn newtype() {
                let data: Node = from_str("true").unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn tuple_struct() {
                let data: Workaround = from_str("<root>42</root><root>answer</root>").unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            mod struct_ {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><float>42</float><string>answer</string></root>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root string="answer"><nested float="42"/></root>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Holder {
                            nested: NestedAttrs { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><float>42</float><string2>answer</string2></root>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string2: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: NodeAttrs =
                        from_str(r#"<root float="42" string2="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        NodeAttrs::Flatten {
                            nested: NestedAttrs { float: "42".into() },
                            string2: "answer".into()
                        }
                    );
                }
            }
        }
    }
}
