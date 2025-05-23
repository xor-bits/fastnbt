//! This module contains a serde serializer for sNBT data.
//! This should be able to serialize most structures to sNBT.
//! Use [`to_vec`](crate::to_vec) or [`to_string`](crate::to_string).
//!
//! Some Rust structures have no sensible mapping to sNBT data.
//! These cases will result in an error (not a panic).
//! If you find a case where you think there is a valid way to serialize it, please open an issue.
//!
//! The [de](crate::de) module contains more information about (de)serialization.
//!
//! ## Uuid
//! Because sNBT is a human-readable format,
//! `Uuid`s are represented as strings.

use std::io::Write;

use serde::ser::{
    self, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::{error::Error, BYTE_ARRAY_TOKEN_STR, INT_ARRAY_TOKEN_STR, LONG_ARRAY_TOKEN_STR};

use self::name_serializer::NameSerializer;

mod array_serializer;
mod name_serializer;

pub(crate) fn write_escaped_str<W: Write>(mut writer: W, v: &str) -> Result<(), Error> {
    writer.write_all(b"\"")?;
    let bytes = v.as_bytes();
    let mut start = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != b'"' && byte != b'\\' {
            continue;
        }
        if start < i {
            writer.write_all(v[start..i].as_bytes())?;
        }
        if byte == b'"' {
            writer.write_all(b"\\\"")?;
        } else if byte == b'\\' {
            writer.write_all(b"\\\\")?;
        }
        start = i + 1;
    }
    if start != bytes.len() {
        writer.write_all(v[start..].as_bytes())?;
    }
    Ok(writer.write_all(b"\"")?)
}

pub struct Serializer<W> {
    pub(crate) writer: W,
    pub(crate) indent: Option<usize>,
}

impl<W: Write> Serializer<W> {
    pub fn newline(&mut self) -> Result<(), Error> {
        if let Some(indent) = self.indent {
            self.writer.write_all(b"\n")?;
            for _ in 0..indent {
                self.writer.write_all(b"    ")?;
            }
        }
        Ok(())
    }

    pub fn push_indent(&mut self) {
        self.indent = self.indent.map(|indent| indent.saturating_add(1));
    }

    pub fn pop_indent(&mut self) {
        self.indent = self.indent.map(|indent| indent.saturating_sub(1));
    }
}

impl<'a, W: 'a + Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ArraySerializer<'a, W>;
    type SerializeTuple = ArraySerializer<'a, W>;
    type SerializeTupleStruct = ArraySerializer<'a, W>;
    type SerializeTupleVariant = ArraySerializer<'a, W>;
    type SerializeMap = CompoundSerializer<'a, W>;
    type SerializeStruct = CompoundSerializer<'a, W>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_all(if v { b"true" } else { b"false" })?)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"b")?)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"s")?)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        Ok(self.writer.write_all(s.as_bytes())?)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"l")?)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"b")?)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"s")?)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        Ok(self.writer.write_all(s.as_bytes())?)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"l")?)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format(v);
        self.writer.write_all(s.as_bytes())?;
        Ok(self.writer.write_all(b"f")?)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format(v);
        Ok(self.writer.write_all(s.as_bytes())?)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_escaped_str(&mut self.writer, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut serializer = ArraySerializer::new("", self)?;
        for byte in v {
            SerializeSeq::serialize_element(&mut serializer, byte)?;
        }
        SerializeSeq::end(serializer)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::bespoke("cannot serialize None".to_string()))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::bespoke("cannot serialize unit: ()".to_string()))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::bespoke(format!(
            "cannot serialize unit struct: {name}"
        )))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(Error::bespoke(
            "cannot serialize newtype variant, please open fastnbt issue".to_string(),
        ))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        ArraySerializer::new("", self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::bespoke(
            "cannot serialize newtype tuple variant, please open fastnbt issue".to_string(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        CompoundSerializer::new(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        CompoundSerializer::new(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::bespoke(
            "cannot serialize struct variant, please open fastnbt issue".to_string(),
        ))
    }
}

pub struct ArraySerializer<'a, W> {
    first: bool,
    serializer: &'a mut Serializer<W>,
    prefix: &'static str,
}

impl<'a, W: Write> ArraySerializer<'a, W> {
    pub fn new(
        prefix: &'static str,
        serializer: &'a mut Serializer<W>,
    ) -> Result<ArraySerializer<'a, W>, Error> {
        Ok(Self {
            first: false,
            serializer,
            prefix,
        })
    }
}

impl<'a, W: Write> SerializeSeq for ArraySerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if !self.first {
            self.first = true;

            self.serializer.writer.write_all(b"[")?;
            self.serializer.push_indent();
            self.serializer.newline()?;
            self.serializer.writer.write_all(self.prefix.as_bytes())?;
        } else {
            self.serializer.writer.write_all(b",")?;
        }
        self.serializer.newline()?;
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.first {
            self.serializer.pop_indent();
            self.serializer.newline()?;
        } else {
            self.serializer.writer.write_all(b"[")?;
            self.serializer.writer.write_all(self.prefix.as_bytes())?;
        }
        Ok(self.serializer.writer.write_all(b"]")?)
    }
}

impl<'a, W: Write + 'a> SerializeTuple for ArraySerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, W: Write + 'a> SerializeTupleStruct for ArraySerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, W: Write + 'a> SerializeTupleVariant for ArraySerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

pub struct CompoundSerializer<'a, W> {
    serializer: &'a mut Serializer<W>,
    is_compound: bool,
    has_first: bool,
    key: Option<Vec<u8>>,
}

impl<'a, W: Write + 'a> CompoundSerializer<'a, W> {
    pub fn new(serializer: &'a mut Serializer<W>) -> Result<CompoundSerializer<'a, W>, Error> {
        Ok(Self {
            serializer,
            is_compound: false,
            has_first: false,
            key: None,
        })
    }
}

impl<'a, W: Write + 'a> SerializeMap for CompoundSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let mut name = Vec::new();
        key.serialize(&mut NameSerializer { name: &mut name })?;
        self.key = Some(name);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let name = self.key.take().ok_or_else(|| {
            Error::bespoke("serialize_value called before serialize_key".to_string())
        })?;

        if !self.has_first {
            self.has_first = true;
        } else {
            self.serializer.writer.write_all(b",")?;
            self.serializer.newline()?;
        }

        match std::str::from_utf8(&name) {
            Ok(BYTE_ARRAY_TOKEN_STR) => value.serialize(array_serializer::ArraySerializer {
                ser: self.serializer,
                stride: 1,
                prefix: "B;",
            }),
            Ok(INT_ARRAY_TOKEN_STR) => value.serialize(array_serializer::ArraySerializer {
                ser: self.serializer,
                stride: 4,
                prefix: "I;",
            }),
            Ok(LONG_ARRAY_TOKEN_STR) => value.serialize(array_serializer::ArraySerializer {
                ser: self.serializer,
                stride: 8,
                prefix: "L;",
            }),
            _ => {
                if !self.is_compound {
                    self.is_compound = true;
                    self.serializer.writer.write_all(b"{")?;
                    self.serializer.push_indent();
                    self.serializer.newline()?;
                }
                self.serializer.writer.write_all(&name)?;
                let sep: &[u8] = if self.serializer.indent.is_some() {
                    b": "
                } else {
                    b":"
                };
                self.serializer.writer.write_all(sep)?;
                value.serialize(&mut *self.serializer)
            }
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.is_compound {
            self.serializer.pop_indent();
            self.serializer.newline()?;
            self.serializer.writer.write_all(b"}")?;
        } else if !self.has_first {
            self.serializer.writer.write_all(b"{}")?;
        }
        Ok(())
    }
}

impl<'a, W: Write + 'a> SerializeStruct for CompoundSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}
