use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub struct Number(serde_json::Number);

impl Number {
    pub fn as_u32(&self) -> Option<u32> {
        self.0.as_u64().and_then(|value| u32::try_from(value).ok())
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

pub enum Json {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl Json {
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn number(value: u32) -> Self {
        Self::Number(Number(value.into()))
    }

    /// Null preserves the rendered shape when an optional field has no value.
    pub fn optional(value: Option<impl Into<String>>) -> Self {
        match value {
            Some(value) => Self::String(value.into()),
            None => Self::Null,
        }
    }

    pub fn array(items: impl IntoIterator<Item = Self>) -> Self {
        Self::Array(items.into_iter().collect())
    }

    pub fn strings(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Array(items.into_iter().map(Self::string).collect())
    }

    pub fn object(fields: impl IntoIterator<Item = (&'static str, Self)>) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect(),
        )
    }

    pub fn map(entries: impl IntoIterator<Item = (String, Self)>) -> Self {
        Self::Object(entries.into_iter().collect())
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    pub fn render(&self) -> String {
        let mut rendered =
            serde_json::to_string_pretty(self).expect("the JSON value model always serialises");
        rendered.push('\n');
        rendered
    }
}

impl Serialize for Json {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.0.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(item)?;
                }
                sequence.end()
            }
            Self::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (name, value) in fields {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JsonVisitor;

        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Json;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_unit<E>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Json, E> {
                Ok(Json::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Json, E> {
                Ok(Json::Number(Number(value.into())))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
                Ok(Json::Number(Number(value.into())))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Json, E>
            where
                E: serde::de::Error,
            {
                let value = serde_json::Number::from_f64(value)
                    .ok_or_else(|| E::custom("expected a finite JSON number"))?;
                Ok(Json::Number(Number(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Json, E> {
                Ok(Json::String(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Json, E> {
                Ok(Json::String(value))
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Json, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = sequence.next_element()? {
                    items.push(item);
                }
                Ok(Json::Array(items))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Json, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut fields = Vec::new();
                while let Some(field) = map.next_entry()? {
                    fields.push(field);
                }
                Ok(Json::Object(fields))
            }
        }

        deserializer.deserialize_any(JsonVisitor)
    }
}

/// Optional-field readers treat explicit null and absence alike.
pub fn field<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
    declared(value, key).filter(|v| !matches!(v, Json::Null))
}

/// Required-field checks use this accessor to distinguish explicit null from absence.
pub fn declared<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
    match value {
        Json::Object(fields) => fields.iter().find(|(name, _)| name == key).map(|(_, v)| v),
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => None,
    }
}

pub fn text(value: &Json, key: &str) -> Option<String> {
    match field(value, key) {
        Some(Json::String(found)) => Some(found.clone()),
        _ => None,
    }
}

pub fn number(value: &Json, key: &str) -> Option<u32> {
    match field(value, key) {
        Some(Json::Number(found)) => found.as_u32(),
        _ => None,
    }
}

pub fn items<'a>(value: &'a Json, key: &str) -> &'a [Json] {
    match field(value, key) {
        Some(Json::Array(found)) => found,
        _ => &[],
    }
}

pub fn strings(value: &Json, key: &str) -> Vec<String> {
    items(value, key)
        .iter()
        .filter_map(|item| match item {
            Json::String(found) => Some(found.clone()),
            Json::Null | Json::Bool(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_null_is_absent() {
        let doc = Json::parse(r#"{"imported": null, "source": "one"}"#).unwrap();
        assert!(field(&doc, "imported").is_none());
        assert!(field(&doc, "missing").is_none());
        assert!(field(&doc, "source").is_some());
    }

    #[test]
    fn valid_number_forms_reach_their_consumers() {
        for text in ["-1", "1.5", "4294967296", "18446744073709551615"] {
            let Json::Number(number) = Json::parse(text).unwrap() else {
                panic!("{text} did not parse as a number");
            };
            assert_eq!(number.to_string(), text);
        }
    }

    #[test]
    fn an_unsigned_32_bit_field_rejects_other_number_forms() {
        for text in ["-1", "1.5", "4294967296"] {
            let doc = Json::parse(&format!(r#"{{"value":{text}}}"#)).unwrap();
            assert_eq!(number(&doc, "value"), None, "{text}");
        }
        let doc = Json::parse(r#"{"value":4294967295}"#).unwrap();
        assert_eq!(number(&doc, "value"), Some(u32::MAX));
    }

    #[test]
    fn escapes_and_surrogate_pairs_read_as_text() {
        let doc = Json::parse(
            r#"{"text":"quote \" slash \\ line\n return\r tab\t back\b form\f solidus\/ nul\u0000 pair\uD83D\uDE00"}"#,
        )
        .unwrap();
        assert_eq!(
            text(&doc, "text").as_deref(),
            Some(
                "quote \" slash \\ line\n return\r tab\t back\u{8} form\u{c} solidus/ nul\0 pair😀"
            )
        );
        for text in [r#""\uD83D""#, r#""\uDE00""#] {
            assert!(Json::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn object_order_and_the_trailing_newline_are_stable() {
        let doc = Json::parse(r#"{"second":2,"first":1}"#).unwrap();
        assert_eq!(doc.render(), "{\n  \"second\": 2,\n  \"first\": 1\n}\n");
    }

    #[test]
    fn duplicate_object_fields_remain_for_the_consumer() {
        let doc = Json::parse(r#"{"value":1,"value":2}"#).unwrap();
        let Json::Object(fields) = &doc else {
            panic!("the document is an object");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(number(&doc, "value"), Some(1));
        assert_eq!(doc.render(), "{\n  \"value\": 1,\n  \"value\": 2\n}\n");
    }

    #[test]
    fn every_value_round_trips_through_the_rendered_document() {
        let doc = Json::object([
            ("none", Json::Null),
            ("yes", Json::Bool(true)),
            ("number", Json::number(7)),
            ("text", Json::string("line\nquote\"")),
            ("list", Json::array([Json::number(1), Json::number(2)])),
            ("object", Json::object([("inside", Json::Bool(false))])),
        ]);
        let rendered = doc.render();
        assert_eq!(Json::parse(&rendered).unwrap().render(), rendered);
    }

    #[test]
    fn trailing_content_is_refused() {
        assert!(Json::parse("null true").is_err());
    }

    #[test]
    fn nesting_has_a_fixed_limit() {
        let accepted = format!("{}0{}", "[".repeat(127), "]".repeat(127));
        let refused = format!("{}0{}", "[".repeat(128), "]".repeat(128));
        assert!(Json::parse(&accepted).is_ok());
        assert!(Json::parse(&refused).is_err());
    }
}
