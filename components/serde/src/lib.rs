extern crate core;

pub mod de;
mod error;
//pub mod ser;

pub use crate::de::{from_reader, from_str, Deserializer};
pub use crate::error::{Error, Result};
//pub use crate::ser::{to_string, to_writer, Serializer};
