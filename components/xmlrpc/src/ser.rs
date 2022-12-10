use crate::{MethodResponse, XmlRpcError};
use serde::ser::{
    Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Serialize, Serializer};
use std::fmt::Display;
use xrs_writer::write::Utf8Writer;
use xrs_writer::{CompactXmlWrite, XmlWrite, XmlWriter};

pub fn method_call_to_string<T>(method_name: &str, params: &T) -> Result<String, XmlRpcError>
where
    T: ?Sized + Serialize,
{
    let mut buf = String::new();
    method_call(method_name, params, CompactXmlWrite::new(&mut buf))?;
    Ok(buf)
}

pub fn method_call_to_writer<W, T>(
    writer: W,
    method_name: &str,
    params: &T,
) -> Result<(), XmlRpcError>
where
    W: std::io::Write,
    T: ?Sized + Serialize,
{
    method_call(
        method_name,
        params,
        CompactXmlWrite::new(Utf8Writer::new(writer)),
    )
}

pub fn method_call<T>(
    method_name: &str,
    params: &T,
    xml_write: impl XmlWrite<Error = std::io::Error>,
) -> Result<(), XmlRpcError>
where
    T: ?Sized + Serialize,
{
    let mut serializer = ParamsSerializer(XmlRpcSerializer {
        writer: XmlWriter::without_decl(xml_write),
    });
    serializer.0.writer.element("methodCall")?.finish()?;

    serializer.0.writer.element("methodName")?.finish()?;
    serializer.0.writer.characters(method_name)?;
    serializer.0.writer.end_element()?;

    params.serialize(&mut serializer)?;

    serializer.0.writer.end_element()?;

    Ok(())
}

pub fn method_response<T>(
    reponse: &MethodResponse<T>,
    xml_write: impl XmlWrite<Error = std::io::Error>,
) -> Result<(), XmlRpcError>
where
    T: Serialize,
{
    let mut serializer = XmlRpcSerializer {
        writer: XmlWriter::without_decl(xml_write),
    };
    serializer.writer.element("methodResponse")?.finish()?;

    match reponse {
        MethodResponse::Success(value) => {
            serializer.writer.element("params")?.finish()?;
            serializer.writer.element("param")?.finish()?;
            serializer.writer.element("value")?.finish()?;
            value.serialize(&mut serializer)?;
            serializer.writer.end_element()?;
            serializer.writer.end_element()?;
            serializer.writer.end_element()?;
        }
        MethodResponse::Fault(fault) => {
            serializer.writer.element("fault")?.finish()?;
            serializer.writer.element("value")?.finish()?;
            fault.serialize(&mut serializer)?;
            serializer.writer.end_element()?;
            serializer.writer.end_element()?;
        }
    }

    serializer.writer.end_element()?;

    Ok(())
}

impl serde::ser::Error for XmlRpcError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        XmlRpcError::new_ser(msg.to_string())
    }
}

struct XmlRpcSerializer<W: XmlWrite> {
    writer: XmlWriter<'static, W>,
}

impl<'w, W: XmlWrite<Error = std::io::Error>> Serializer for &'w mut XmlRpcSerializer<W> {
    type Ok = ();
    type Error = XmlRpcError;
    type SerializeSeq = ArraySerializer<'w, W>;
    type SerializeTuple = ArraySerializer<'w, W>;
    type SerializeTupleStruct = ArraySerializer<'w, W>;
    type SerializeTupleVariant = ArraySerializer<'w, W>;
    type SerializeMap = StructSerializer<'w, W>;
    type SerializeStruct = StructSerializer<'w, W>;
    type SerializeStructVariant = StructSerializer<'w, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.writer.element("boolean")?.finish()?;
        self.writer.characters(if v { "1" } else { "0" })?;
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.writer.element("i4")?.finish()?;
        self.writer.characters(&v.to_string())?; // TODO: write to buffer
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        panic!("64-bit integer not supported")
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        panic!("unsigned 32-bit integer not supported")
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        panic!("unsigned 64-bit integer not supported")
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.writer.element("double")?.finish()?;
        self.writer.characters(&v.to_string())?; // TODO: write to buffer
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.writer.element("string")?.finish()?;
        self.writer.characters(v.encode_utf8(&mut buf))?;
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.writer.element("string")?.finish()?;
        self.writer.characters(v)?;
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.writer.element("base64")?.finish()?;
        self.writer.characters(&base64::encode(v))?;
        self.writer.end_element()?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.writer.element("nil")?.finish_empty()?;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        ArraySerializer::start(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        ArraySerializer::start(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        ArraySerializer::start(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        ArraySerializer::start(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        StructSerializer::start(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        StructSerializer::start(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        StructSerializer::start(self)
    }
}

pub struct ArraySerializer<'s, W: XmlWrite> {
    ser: &'s mut XmlRpcSerializer<W>,
}

impl<'s, W: XmlWrite<Error = std::io::Error>> ArraySerializer<'s, W> {
    fn start(ser: &'s mut XmlRpcSerializer<W>) -> Result<Self, XmlRpcError> {
        ser.writer.element("array")?.finish()?;
        ser.writer.element("data")?.finish()?;
        Ok(Self { ser })
    }

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), XmlRpcError>
    where
        T: Serialize,
    {
        self.ser.writer.element("value")?.finish()?;
        value.serialize(&mut *self.ser)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn end(self) -> Result<(), XmlRpcError> {
        self.ser.writer.end_element()?;
        self.ser.writer.end_element()?;
        Ok(())
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeSeq for ArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTuple for ArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTupleVariant for ArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTupleStruct for ArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

pub struct StructSerializer<'s, W: XmlWrite> {
    ser: &'s mut XmlRpcSerializer<W>,
}

impl<'s, W: XmlWrite<Error = std::io::Error>> StructSerializer<'s, W> {
    fn start(ser: &'s mut XmlRpcSerializer<W>) -> Result<Self, XmlRpcError> {
        ser.writer.element("struct")?.finish()?;
        Ok(Self { ser })
    }

    fn serialize_member<T: ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), XmlRpcError>
    where
        T: Serialize,
    {
        self.ser.writer.element("member")?.finish()?;
        self.serialize_member_name(name)?;
        self.serialize_value(value)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn serialize_member_name(&mut self, name: &'static str) -> Result<(), XmlRpcError> {
        self.ser.writer.element("name")?.finish()?;
        self.ser.writer.characters(name)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), XmlRpcError> {
        self.ser.writer.element("name")?.finish()?;
        let mut buf = String::new();
        key.serialize(MemberNameSerializer(&mut buf))?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), XmlRpcError> {
        self.ser.writer.element("value")?.finish()?;
        value.serialize(&mut *self.ser)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn end(self) -> Result<(), XmlRpcError> {
        self.ser.writer.end_element()?;
        Ok(())
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeStruct for StructSerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_member(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeMap for StructSerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.ser.writer.element("member")?.finish()?;
        self.serialize_key(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_value(value)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        self.ser.writer.element("member")?.finish()?;
        self.serialize_key(key)?;
        self.serialize_value(value)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.writer.end_element()?;
        Ok(())
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeStructVariant for StructSerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_member(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

struct MemberNameSerializer<'a>(&'a mut String);

impl<'a> MemberNameSerializer<'a> {
    pub fn new(buf: &'a mut String) -> Self {
        Self(buf)
    }

    fn not_a_string(got: &str) -> XmlRpcError {
        XmlRpcError::new_ser(format!(
            "expected string as struct member name, got {:?}",
            got
        ))
    }
}

impl<'a> Serializer for MemberNameSerializer<'a> {
    type Ok = ();
    type Error = XmlRpcError;
    type SerializeSeq = Impossible<(), Self::Error>;
    type SerializeTuple = Impossible<(), Self::Error>;
    type SerializeTupleStruct = Impossible<(), Self::Error>;
    type SerializeTupleVariant = Impossible<(), Self::Error>;
    type SerializeMap = Impossible<(), Self::Error>;
    type SerializeStruct = Impossible<(), Self::Error>;
    type SerializeStructVariant = Impossible<(), Self::Error>;

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("bool"))
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.0.push_str(v);
        Ok(())
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("bytes"))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("option"))
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.serialize_none()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("unit"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("unit struct"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_string("unit variant"))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Err(Self::not_a_string("newtype struct"))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Err(Self::not_a_string("new type variant"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Self::not_a_string("i8"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Self::not_a_string("tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Self::not_a_string("tuple struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Self::not_a_string("tuple variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Self::not_a_string("map"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Self::not_a_string("struct"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Self::not_a_string("struct variant"))
    }
}

struct ParamsSerializer<W: XmlWrite>(XmlRpcSerializer<W>);

impl<W: XmlWrite<Error = std::io::Error>> ParamsSerializer<W> {
    fn serialize_single_arg<T: Serialize>(&mut self, value: T) -> Result<(), XmlRpcError> {
        self.0.writer.element("params")?.finish()?;
        self.0.writer.element("param")?.finish()?;
        value.serialize(&mut self.0)?;
        self.0.writer.end_element()?;
        self.0.writer.end_element()?;
        Ok(())
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> Serializer for &'s mut ParamsSerializer<W> {
    type Ok = ();
    type Error = XmlRpcError;
    type SerializeSeq = ParamsArraySerializer<'s, W>;
    type SerializeTuple = ParamsArraySerializer<'s, W>;
    type SerializeTupleStruct = ParamsArraySerializer<'s, W>;
    type SerializeTupleVariant = ParamsArraySerializer<'s, W>;
    type SerializeMap = ParamsArraySerializer<'s, W>;
    type SerializeStruct = ParamsArraySerializer<'s, W>;
    type SerializeStructVariant = ParamsArraySerializer<'s, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_single_arg(())
    }

    fn serialize_some<T: ?Sized>(self, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.serialize_single_arg(v)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.writer.element("params")?.finish()?;
        self.0.writer.end_element()?;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.serialize_single_arg(v)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.serialize_single_arg(v)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        ParamsArraySerializer::start(&mut self.0)
    }
}

pub struct ParamsArraySerializer<'s, W: XmlWrite> {
    ser: &'s mut XmlRpcSerializer<W>,
}

impl<'s, W: XmlWrite<Error = std::io::Error>> ParamsArraySerializer<'s, W> {
    fn start(ser: &'s mut XmlRpcSerializer<W>) -> Result<Self, XmlRpcError> {
        ser.writer.element("params")?.finish()?;
        Ok(Self { ser })
    }

    fn serialize_param<T: ?Sized>(&mut self, value: &T) -> Result<(), XmlRpcError>
    where
        T: Serialize,
    {
        self.ser.writer.element("param")?.finish()?;
        value.serialize(&mut *self.ser)?;
        self.ser.writer.end_element()?;
        Ok(())
    }

    fn end(self) -> Result<(), XmlRpcError> {
        self.ser.writer.end_element()?;
        Ok(())
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeSeq for ParamsArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTuple for ParamsArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTupleVariant
    for ParamsArraySerializer<'s, W>
{
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeTupleStruct
    for ParamsArraySerializer<'s, W>
{
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeStruct for ParamsArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeMap for ParamsArraySerializer<'s, W> {
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        _key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<'s, W: XmlWrite<Error = std::io::Error>> SerializeStructVariant
    for ParamsArraySerializer<'s, W>
{
    type Ok = ();
    type Error = XmlRpcError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_param(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}
