use serde::ser::{
    Error, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::Serialize;

use serde::export::PhantomData;

pub struct NonSerializer<Ok, Err> {
    __non_constructable: (PhantomData<Ok>, PhantomData<Err>),
}

impl<Ok, Err: Error> SerializeSeq for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeTuple for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeTupleStruct for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeTupleVariant for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeMap for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeStruct for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<Ok, Err: Error> SerializeStructVariant for NonSerializer<Ok, Err> {
    type Ok = Ok;
    type Error = Err;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}
