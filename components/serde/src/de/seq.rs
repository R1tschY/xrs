use std::borrow::Cow;

use serde::de;

use crate::de::Deserializer;
use crate::Error;

/// A SeqAccess
pub struct SeqAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    max_size: Option<usize>,
    name: Option<Cow<'a, str>>,
}

impl<'a, 'de> SeqAccess<'a, 'de> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de>, max_size: Option<usize>) -> Result<Self, Error> {
        Ok(SeqAccess {
            de,
            max_size,
            name: None,
        })
    }
}

impl<'de, 'a> de::SeqAccess<'de> for SeqAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if let Some(s) = self.max_size.as_mut() {
            if *s == 0 {
                return Ok(None);
            }
            *s -= 1;
        }
        if let Some(name) = &self.name {
            match self.de.next_maybe_start()? {
                Some(tag) => {
                    if &tag.name == name {
                        seed.deserialize(&mut *self.de).map(Some)
                    } else {
                        Ok(None)
                    }
                }
                None => Ok(None),
            }
        } else {
            self.name = Some(self.de.reader.top_name().unwrap().to_string().into());
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.max_size
    }
}
