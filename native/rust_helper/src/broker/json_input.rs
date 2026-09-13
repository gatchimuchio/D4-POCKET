//! 内容を含む全階層で重複fieldを拒否するIPCの構造読取。権限判定は呼出し元が所有する。
use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) fn read_unique<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, serde_json::Error> {
    use serde::de::{self, MapAccess, SeqAccess, Visitor};
    struct 一意値(Value);
    impl<'de> Deserialize<'de> for 一意値 {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct 検査;
            impl<'de> Visitor<'de> for 検査 {
                type Value = 一意値;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("重複のないJSON")
                }
                fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Self::Value, M::Error> {
                    let mut v = serde_json::Map::new();
                    while let Some((k, x)) = m.next_entry::<String, 一意値>()? {
                        if v.insert(k, x.0).is_some() {
                            return Err(de::Error::custom("重複field"));
                        }
                    }
                    Ok(一意値(Value::Object(v)))
                }
                fn visit_seq<S: SeqAccess<'de>>(self, mut s: S) -> Result<Self::Value, S::Error> {
                    let mut v = Vec::new();
                    while let Some(x) = s.next_element::<一意値>()? {
                        v.push(x.0);
                    }
                    Ok(一意値(Value::Array(v)))
                }
                fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                    Ok(一意値(json!(v)))
                }
                fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                    Ok(一意値(json!(v)))
                }
                fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                    Ok(一意値(json!(v)))
                }
                fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                    Ok(一意値(json!(v)))
                }
                fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                    Ok(一意値(json!(v)))
                }
                fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                    Ok(一意値(Value::Null))
                }
            }
            d.deserialize_any(検査)
        }
    }
    let value: 一意値 = serde_json::from_str(raw)?;
    serde_json::from_value(value.0)
}
