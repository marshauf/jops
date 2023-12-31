use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

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

// Get element of arrray from right side without panic.
fn get_right(array: &Vec<Value>, i: usize) -> Option<&Value> {
    if array.len() < i {
        None
    } else {
        array.get(array.len() - i)
    }
}

// Get mutable element of array from right side without panic.
fn get_right_mut(array: &mut Vec<Value>, i: usize) -> Option<&mut Value> {
    if array.len() < i {
        None
    } else {
        let i = array.len() - i;
        array.get_mut(i)
    }
}

impl Deref for JsonPath {
    type Target = Vec<JsonPathElement>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JsonPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl JsonPath {
    pub fn find<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let mut value = value;
        for e in self.iter() {
            let sub = match e {
                JsonPathElement::Field(key) => value.get(key),
                JsonPathElement::Index(JsonPathIndex::NthLefth(i)) => value.get(i),
                JsonPathElement::Index(JsonPathIndex::NthRight(i)) => {
                    value.as_array().and_then(|a| get_right(a, *i))
                }
            };
            if let Some(sub) = sub {
                value = sub;
            } else {
                return None;
            }
        }
        Some(value)
    }

    pub fn find_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        let mut value = value;
        for e in self.iter() {
            let sub = match e {
                JsonPathElement::Field(key) => value.get_mut(key),
                JsonPathElement::Index(JsonPathIndex::NthLefth(i)) => value.get_mut(i),
                JsonPathElement::Index(JsonPathIndex::NthRight(i)) => {
                    value.as_array_mut().and_then(|a| get_right_mut(a, *i))
                }
            };
            if let Some(sub) = sub {
                value = sub;
            } else {
                return None;
            }
        }
        Some(value)
    }

    pub fn insert<'a>(&self, value: &'a mut Value, v: Value) -> Option<&'a Value> {
        match self.find_last_mut(value) {
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthLefth(i)))) => {
                let i = *i;
                if i <= target.len() {
                    target.insert(i, v);
                    Some(value)
                } else {
                    None
                }
            }
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthRight(i)))) => {
                if target.len() < *i {
                    return None;
                }
                let i = target.len() - i;
                if i <= target.len() {
                    target.insert(i, v);
                    Some(value)
                } else {
                    None
                }
            }
            Some((Value::Object(target), JsonPathElement::Field(key))) => {
                if target.contains_key(key) {
                    None
                } else {
                    target.insert(key.clone(), v);
                    Some(value)
                }
            }
            _ => None,
        }
    }

    pub fn replace<'a>(&self, value: &'a mut Value, v: Value) -> Option<&'a Value> {
        if let Some(target) = self.find_mut(value) {
            *target = v;
            Some(value)
        } else {
            None
        }
    }

    pub fn set<'a>(&self, value: &'a mut Value, v: Value) -> Option<&'a Value> {
        match self.find_last_mut(value) {
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthLefth(i)))) => {
                if let Some(target) = target.get_mut(*i) {
                    *target = v;
                    Some(value)
                } else {
                    None
                }
            }
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthRight(i)))) => {
                if let Some(target) = get_right_mut(target, *i) {
                    *target = v;
                    Some(value)
                } else {
                    None
                }
            }
            Some((Value::Object(target), JsonPathElement::Field(key))) => {
                target.insert(key.clone(), v);
                Some(value)
            }
            _ => None,
        }
    }

    pub fn remove<'a>(&self, value: &'a mut Value) -> Option<&'a Value> {
        match self.find_last_mut(value) {
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthLefth(i)))) => {
                if target.len() < *i {
                    None
                } else {
                    target.remove(*i);
                    Some(value)
                }
            }
            Some((Value::Array(target), JsonPathElement::Index(JsonPathIndex::NthRight(i)))) => {
                if target.len() < *i {
                    None
                } else {
                    let i = target.len() - i;
                    target.remove(i);
                    Some(value)
                }
            }
            Some((Value::Object(target), JsonPathElement::Field(key))) => {
                if target.remove(key).is_some() {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn find_last_mut<'a>(&self, value: &'a mut Value) -> Option<(&'a mut Value, &JsonPathElement)> {
        self.split_last().and_then(|(last, rest)| {
            JsonPath(rest.to_vec())
                .find_mut(value)
                .map(|target| (target, last))
        })
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
                let index =
                    JsonPathElement::Index(JsonPathIndex::NthLefth(field.parse().unwrap_or(0)));
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
                    let index = if iter.next_if_eq(&BEGIN_REVERSE_INDEX).is_some() {
                        iter.next_if_eq(&'-');
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
                            JsonPathIndex::NthLefth(field.parse().unwrap_or(0))
                        }
                        JsonPathIndex::NthRight(_) => {
                            JsonPathIndex::NthRight(field.parse().unwrap_or(0))
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
    fn path_mut(&'a mut self, query: &str) -> Result<&'a mut Value, &'static str>;
}

impl<'a> JsonPathQuery<'a> for Value {
    #[inline]
    fn path(&'a self, query: &str) -> Result<&'a Value, &'static str> {
        let path = JsonPath::try_from(query)?;
        path.find(self).ok_or("unable to find path to value")
    }

    #[inline]
    fn path_mut(&'a mut self, query: &str) -> Result<&'a mut Value, &'static str> {
        let path = JsonPath::try_from(query)?;
        path.find_mut(self).ok_or("unable to find path to value")
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
            (
                "$.a[#]",
                Ok(JsonPath(vec![
                    JsonPathElement::Field("a".to_string()),
                    JsonPathElement::Index(JsonPathIndex::NthRight(0)),
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
            ("$[2]", json!([1]), Err("unable to find path to value")),
            (
                "$.a[#-2]",
                json!({ "b": [1] }),
                Err("unable to find path to value"),
            ),
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
            let mut value = value;
            assert_eq!(
                value.path_mut(path).cloned(),
                expected,
                "expected {} from {} to be {:?}",
                value,
                path,
                expected
            );
        }
    }

    #[test]
    fn insert() {
        let tests: Vec<(
            JsonPath,
            serde_json::Value,
            serde_json::Value,
            Option<serde_json::Value>,
        )> = vec![
            (
                "$.a".try_into().unwrap(),
                json!({}),
                json!("test"),
                Some(json!({ "a": "test"})),
            ),
            (
                "$.a.b[1]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, "test", 2, 4]}})),
            ),
            (
                "$.a.b[#]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, 2, 4, "test"]}})),
            ),
            (
                "$.a.b[#-3]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": ["test", 1, 2, 4 ]}})),
            ),
            (
                "$.a".try_into().unwrap(),
                json!({"a": 10.0}),
                json!("test"),
                None,
            ),
            (
                "$.a[1]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
            (
                "$.a[#-3]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
        ];

        for (path, mut value, extra, expected) in tests {
            let value = path.insert(&mut value, extra);
            assert_eq!(
                value,
                expected.as_ref(),
                "expected {:?} to be {:?}",
                value,
                expected
            );
        }
    }

    #[test]
    fn replace() {
        let tests: Vec<(
            JsonPath,
            serde_json::Value,
            serde_json::Value,
            Option<serde_json::Value>,
        )> = vec![
            ("$.a".try_into().unwrap(), json!({}), json!("test"), None),
            (
                "$.a.b[1]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, "test", 4]}})),
            ),
            (
                "$.a.b[#-2]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, "test", 4 ]}})),
            ),
            (
                "$.a".try_into().unwrap(),
                json!({"a": 10.0}),
                json!("test"),
                Some(json!({"a": "test"})),
            ),
            (
                "$.a[1]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
            (
                "$.a[#-3]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
        ];

        for (path, mut value, extra, expected) in tests {
            let value = path.replace(&mut value, extra);
            assert_eq!(
                value,
                expected.as_ref(),
                "expected {:?} to be {:?}",
                value,
                expected
            );
        }
    }

    #[test]
    fn set() {
        let tests: Vec<(
            JsonPath,
            serde_json::Value,
            serde_json::Value,
            Option<serde_json::Value>,
        )> = vec![
            (
                "$.a".try_into().unwrap(),
                json!({}),
                json!("test"),
                Some(json!({ "a": "test" })),
            ),
            (
                "$.a.b[1]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, "test", 4]}})),
            ),
            (
                "$.a.b[#-2]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                json!("test"),
                Some(json!({ "a": { "b": [1, "test", 4 ]}})),
            ),
            (
                "$.a".try_into().unwrap(),
                json!({"a": 10.0}),
                json!("test"),
                Some(json!({"a": "test"})),
            ),
            (
                "$.a[1]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
            (
                "$.a[#-3]".try_into().unwrap(),
                json!({"a": []}),
                json!("test"),
                None,
            ),
        ];

        for (path, mut value, extra, expected) in tests {
            let value = path.set(&mut value, extra);
            assert_eq!(
                value,
                expected.as_ref(),
                "expected {:?} to be {:?}",
                value,
                expected
            );
        }
    }

    #[test]
    fn remove() {
        let tests: Vec<(JsonPath, serde_json::Value, Option<serde_json::Value>)> = vec![
            ("$.a".try_into().unwrap(), json!({}), None),
            (
                "$.a.b[1]".try_into().unwrap(),
                json!({"a": { "b": [1,2,8] }}),
                Some(json!({ "a": { "b": [1, 8]}})),
            ),
            (
                "$.a.b[#-2]".try_into().unwrap(),
                json!({"a": { "b": [1,2,4] }}),
                Some(json!({ "a": { "b": [1, 4 ]}})),
            ),
            (
                "$.a".try_into().unwrap(),
                json!({"a": 10.0}),
                Some(json!({})),
            ),
            ("$.a[1]".try_into().unwrap(), json!({"a": []}), None),
            ("$.a[#-3]".try_into().unwrap(), json!({"a": []}), None),
        ];

        for (path, mut value, expected) in tests {
            let value = path.remove(&mut value);
            assert_eq!(
                value,
                expected.as_ref(),
                "expected {:?} to be {:?}",
                value,
                expected
            );
        }
    }
}
