// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

use std::fmt;
use std::marker::PhantomData;

pub const FIELD: &str = "$__v8_magic_value";
pub const NAME: &str = "$__v8_magic_Value";

/// serde_v8::Value allows passing through `v8::Value`s untouched
/// when encoding/decoding and allows mixing rust & v8 values in
/// structs, tuples...
/// The implementation mainly breaks down to:
/// 1. Transmuting between u64 <> serde_v8::Value
/// 2. Using special struct/field names to detect these values
/// 3. Then serde "boilerplate"
pub struct Value<'s> {
  pub v8_value: v8::Local<'s, v8::Value>,
}

impl<'s> From<v8::Local<'s, v8::Value>> for Value<'s> {
  fn from(v8_value: v8::Local<'s, v8::Value>) -> Self {
    Self { v8_value }
  }
}

impl<'s> From<Value<'s>> for v8::Local<'s, v8::Value> {
  fn from(v: Value<'s>) -> Self {
    v.v8_value
  }
}

macro_rules! serialize {
  ($t:tt) => {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      use serde::ser::SerializeStruct;

      let mut s = serializer.serialize_struct(NAME, 1)?;
      let mv = Value {
        v8_value: self.v8_value,
      };
      let hack: $t = unsafe { std::mem::transmute(mv) };
      s.serialize_field(FIELD, &hack)?;
      s.end()
    }
  };
}

impl serde::Serialize for Value<'_> {
  #[cfg(target_pointer_width = "64")]
  serialize!(u64);

  #[cfg(target_pointer_width = "32")]
  serialize!(u32);
}

impl<'de, 's> serde::Deserialize<'de> for Value<'s> {
  fn deserialize<D>(deserializer: D) -> Result<Value<'s>, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    struct ValueVisitor<'s> {
      p1: PhantomData<&'s ()>,
    }

    impl<'de, 's> serde::de::Visitor<'de> for ValueVisitor<'s> {
      type Value = Value<'s>;

      fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a v8::Value")
      }

      #[cfg(target_pointer_width = "64")]
      fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        let mv: Value<'s> = unsafe { std::mem::transmute(v) };
        Ok(mv)
      }

      #[cfg(target_pointer_width = "32")]
      fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        let mv: Value<'s> = unsafe { std::mem::transmute(v) };
        Ok(mv)
      }
    }

    static FIELDS: [&str; 1] = [FIELD];
    let visitor = ValueVisitor { p1: PhantomData };
    deserializer.deserialize_struct(NAME, &FIELDS, visitor)
  }
}
