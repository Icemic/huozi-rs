//! 让纯 unit 枚举复用 serde 的表示实现 `FromStr`。

use serde::de::value::EnumAccessDeserializer;
use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor,
};

/// 用 serde 的枚举表示解析单个字符串。
///
/// 只接受 unit 变体；带 payload 的变体没有可用的字符串表示，返回错误。
pub(crate) fn from_str<'de, T>(value: &'de str) -> Result<T, String>
where
    T: Deserialize<'de>,
{
    T::deserialize(EnumAccessDeserializer::new(StrEnumAccess { value }))
        .map_err(|error| error.to_string())
}

struct StrEnumAccess<'de> {
    value: &'de str,
}

impl<'de> EnumAccess<'de> for StrEnumAccess<'de> {
    type Error = de::value::Error;
    type Variant = UnitVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.value.into_deserializer())?;
        Ok((variant, UnitVariantAccess))
    }
}

struct UnitVariantAccess;

impl<'de> VariantAccess<'de> for UnitVariantAccess {
    type Error = de::value::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

/// 为纯 unit 枚举实现 `FromStr`，字符串取值与 serde 的 `rename_all` 保持一致。
macro_rules! impl_from_str_via_serde {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl std::str::FromStr for $ty {
                type Err = String;

                fn from_str(value: &str) -> Result<Self, Self::Err> {
                    $crate::de_str::from_str(value)
                }
            }
        )+
    };
}

pub(crate) use impl_from_str_via_serde;
