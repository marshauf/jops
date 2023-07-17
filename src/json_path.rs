use std::str::FromStr;

use serde_json::Value;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsonPath(Vec<JsonPathElement>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum JsonPathElement {
    Field(String), // key of an object
    Index(JsonPathIndex),
}

impl ToString for JsonPathElement {
    fn to_string(&self) -> String {
        match self {
            JsonPathElement::Field(v) => v.clone(),
            JsonPathElement::Index(i) => i.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JsonPathIndex {
    NthLefth(usize), // N-th element from zero
    NthRight(usize), // # represents the length of the array, #-1 is the last element
}

impl ToString for JsonPathIndex {
    fn to_string(&self) -> String {
        match self {
            JsonPathIndex::NthLefth(i) => i.to_string(),
            JsonPathIndex::NthRight(i) => format!("#-{i}"),
        }
    }
}

const ROOT: char = '$';
const DOT: char = '.';
const BEGIN_INDEX: char = '[';
const CLOSE_INDEX: char = ']';
const BEGIN_REVERSE_INDEX: char = '#';

impl JsonPath {
    #[inline]
    pub fn last(&self) -> Option<&JsonPathElement> {
        self.0.last()
    }

    pub fn find<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let mut value = value;
        for e in &self.0 {
            value = match e {
                JsonPathElement::Field(key) => value.get(key).unwrap_or(&Value::Null),
                JsonPathElement::Index(JsonPathIndex::NthLefth(i)) => {
                    value.get(i).unwrap_or(&Value::Null)
                }
                JsonPathElement::Index(JsonPathIndex::NthRight(i)) => value
                    .as_array()
                    .map_or(&Value::Null, |a| a.get(a.len() - i).unwrap_or(&Value::Null)),
            };
        }
        Some(value)
    }
}

impl FromStr for JsonPath {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut iter = value.chars().peekable();

        match iter.peek() {
            Some(&ROOT) => _ = iter.next(),
            Some(v) if v.is_numeric() => {
                let mut field = String::new();
                while let Some(c) = iter.next_if(|c| c.is_numeric()) {
                    field.push(c);
                }
                let index = JsonPathElement::Index(JsonPathIndex::NthLefth(field.parse().unwrap()));
                return Ok(JsonPath(vec![index]));
            }
            _ => return Err("expected $ or numeric"),
        };

        let mut path: Vec<JsonPathElement> = Vec::new();
        loop {
            match iter.next() {
                Some(DOT) => {
                    let mut field: String = String::new();
                    while let Some(c) = iter.next_if(|c| c.is_alphabetic()) {
                        field.push(c);
                    }
                    path.push(JsonPathElement::Field(field));
                }
                Some(BEGIN_INDEX) => {
                    let index = if iter.next_if_eq(&BEGIN_REVERSE_INDEX).is_some()
                        && iter.next_if_eq(&'-').is_some()
                    {
                        JsonPathIndex::NthRight(0)
                    } else {
                        JsonPathIndex::NthLefth(0)
                    };

                    let mut field: String = String::new();
                    while let Some(c) = iter.next_if(|c| c.is_numeric()) {
                        field.push(c);
                    }
                    if iter.next_if_eq(&CLOSE_INDEX).is_none() {
                        return Err("expected ]");
                    }
                    let index = match index {
                        JsonPathIndex::NthLefth(_) => {
                            JsonPathIndex::NthLefth(field.parse().unwrap())
                        }
                        JsonPathIndex::NthRight(_) => {
                            JsonPathIndex::NthRight(field.parse().unwrap())
                        }
                    };
                    path.push(JsonPathElement::Index(index));
                }
                None => return Ok(JsonPath(path)),
                _ => return Err("expected . or ["),
            }
        }
    }
}

impl TryFrom<&str> for JsonPath {
    type Error = &'static str;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        JsonPath::from_str(value)
    }
}

pub trait JsonPathQuery<'a> {
    fn path(&'a self, query: &str) -> Result<&'a Value, &'static str>;
}

impl<'a> JsonPathQuery<'a> for Value {
    #[inline]
    fn path(&'a self, query: &str) -> Result<&'a Value, &'static str> {
        let path = JsonPath::try_from(query)?;
        path.find(self).ok_or("unable to find path to value")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn try_from() {
        let tests = vec![
            ("$", Ok(JsonPath(vec![]))),
            (
                "3",
                Ok(JsonPath(vec![JsonPathElement::Index(
                    JsonPathIndex::NthLefth(3),
                )])),
            ),
            (
                "$.a",
                Ok(JsonPath(vec![JsonPathElement::Field("a".to_string())])),
            ),
            (
                "$.a.b",
                Ok(JsonPath(vec![
                    JsonPathElement::Field("a".to_string()),
                    JsonPathElement::Field("b".to_string()),
                ])),
            ),
            (
                "$.abc.bc.cbc",
                Ok(JsonPath(vec![
                    JsonPathElement::Field("abc".to_string()),
                    JsonPathElement::Field("bc".to_string()),
                    JsonPathElement::Field("cbc".to_string()),
                ])),
            ),
            (
                "$[4]",
                Ok(JsonPath(vec![JsonPathElement::Index(
                    JsonPathIndex::NthLefth(4),
                )])),
            ),
            (
                "$[4][3]",
                Ok(JsonPath(vec![
                    JsonPathElement::Index(JsonPathIndex::NthLefth(4)),
                    JsonPathElement::Index(JsonPathIndex::NthLefth(3)),
                ])),
            ),
            (
                "$.a[4].b[3]",
                Ok(JsonPath(vec![
                    JsonPathElement::Field("a".to_string()),
                    JsonPathElement::Index(JsonPathIndex::NthLefth(4)),
                    JsonPathElement::Field("b".to_string()),
                    JsonPathElement::Index(JsonPathIndex::NthLefth(3)),
                ])),
            ),
            (
                "$.a[#-4].b[3]",
                Ok(JsonPath(vec![
                    JsonPathElement::Field("a".to_string()),
                    JsonPathElement::Index(JsonPathIndex::NthRight(4)),
                    JsonPathElement::Field("b".to_string()),
                    JsonPathElement::Index(JsonPathIndex::NthLefth(3)),
                ])),
            ),
            // Invalid
            (".a", Err("expected $ or numeric")),
            ("a", Err("expected $ or numeric")),
            ("[0]", Err("expected $ or numeric")),
            ("$0]", Err("expected . or [")),
        ];
        for (path, expected) in tests {
            assert_eq!(
                path.try_into(),
                expected,
                "expected {} to be {:?}",
                path,
                expected
            );
        }
    }

    #[test]
    fn path() {
        let tests: Vec<(&str, serde_json::Value, Result<serde_json::Value, _>)> = vec![
            ("$", json!({}), Ok(json!({}))),
            ("$.a", json!({"a":"example"}), Ok(json!("example"))),
            ("$[0]", json!([0, 1, 2, 3]), Ok(json!(0))),
            ("$[#-1]", json!([0, 1, 2, 3]), Ok(json!(3))),
            (
                "$.a[#-1].b[0].test",
                json!({"a": [
                    {
                        "b": "invalid"
                    },
                    {
                        "b": [{ "test": "example"}, { "test": "invalid" }]
                    }
                ],
                "b": "invalid"
                }),
                Ok(json!("example")),
            ),
            ("1", json!([1, 2, 4]), Ok(json!(2))),
        ];

        for (path, value, expected) in tests {
            assert_eq!(
                value.path(path).cloned(),
                expected,
                "expected {} from {} to be {:?}",
                value,
                path,
                expected
            );
        }
    }
}
