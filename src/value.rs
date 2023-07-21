use std::{cmp::Ordering, mem::size_of_val, ops::Deref};

use serde_json::Value;

/// Compares two `serde_json::Value`s.
///
/// Follows SQL JSON Operators.
/// Comparing any Value with `Value::Null` returns None.
/// `Value::Bool` is casted to a f64, when comparing with `Value::Number`.
/// `Value::Bool` is always less than a String, Array, or Object.
/// `Value::Number` is always less than a String, Array, or Object.
/// `Value::String` is always less than an Array, or Object.
/// Comparing a `Value::String` with a `Value::Number` trys to parse the String as a f64 for
/// comparison.
/// Arrays and Objects get compared by memory.
pub fn partial_cmp(a: &Value, b: &Value) -> Option<Ordering> {
    if a == b {
        return Some(Ordering::Equal);
    }
    match (a, b) {
        // Equal types
        // Anything with Null can't be compared
        (Value::Null | _, Value::Null) | (Value::Null, _) => None,
        (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
        (Value::Number(a), Value::Number(b)) => {
            // Try to be as precise as possible
            if let (Some(a), Some(ref b)) = (a.as_i64(), b.as_i64()) {
                a.partial_cmp(b)
            } else if let (Some(a), Some(ref b)) = (a.as_u64(), b.as_u64()) {
                a.partial_cmp(b)
            } else if let (Some(a), Some(ref b)) = (a.as_f64(), b.as_f64()) {
                a.partial_cmp(b)
            } else {
                None
            }
        }
        (Value::String(a), Value::String(b)) => a.partial_cmp(b),

        // Unequal types with casting
        (Value::Number(a), Value::Bool(b)) => {
            a.as_f64().and_then(|ref a| a.partial_cmp(&f64::from(*b)))
        }
        (Value::Bool(a), Value::Number(b)) => {
            b.as_f64().and_then(|ref b| f64::from(*a).partial_cmp(b))
        }
        // Bool is always less than a String, Array, Object
        (Value::Bool(_), _) => Some(Ordering::Less),
        (_, Value::Bool(_)) => Some(Ordering::Greater),
        // Try to convert String to f64
        (Value::Number(a), Value::String(b)) => {
            let b: Result<f64, _> = b.parse();
            if let (Some(a), Ok(ref b)) = (a.as_f64(), b) {
                a.partial_cmp(b)
            } else {
                Some(Ordering::Less)
            }
        }
        (Value::String(a), Value::Number(b)) => {
            let a: Result<f64, _> = a.parse();
            if let (Some(ref b), Ok(a)) = (b.as_f64(), a) {
                a.partial_cmp(b)
            } else {
                Some(Ordering::Less)
            }
        }

        // Integer or Real values are less than String, Array, Object
        (Value::Number(_), _) => Some(Ordering::Less),
        (_, Value::Number(_)) => Some(Ordering::Greater),
        // String values are less than Array, Object
        (Value::String(_), _) => Some(Ordering::Less),
        (_, Value::String(_)) => Some(Ordering::Greater),

        // Compare Arrays and Objects by memory size
        (Value::Array(a), Value::Array(b)) => size_of_val(a).partial_cmp(&size_of_val(b)),
        (Value::Array(a), Value::Object(b)) => size_of_val(a).partial_cmp(&size_of_val(b)),
        (Value::Object(a), Value::Array(b)) => size_of_val(a).partial_cmp(&size_of_val(b)),
        (Value::Object(a), Value::Object(b)) => size_of_val(a).partial_cmp(&size_of_val(b)),
    }
}

/// Wraps a reference to a `serde_json::Value` and provides `PartialOrd` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonValue<'a>(&'a Value);

impl<'a> JsonValue<'a> {
    pub fn new(value: &'a Value) -> Self {
        JsonValue(value)
    }
}

impl<'a> PartialOrd for JsonValue<'a> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        partial_cmp(self.0, other.0)
    }
}

impl<'a> From<&'a Value> for JsonValue<'a> {
    #[inline]
    fn from(value: &'a Value) -> Self {
        JsonValue(value)
    }
}

impl<'a> Deref for JsonValue<'a> {
    type Target = serde_json::Value;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_partial_cmp() {
        let tests = vec![
            // Null
            (Value::Null, Value::Null, Some(Ordering::Equal)),
            (json!(0), Value::Null, None),
            (json!(1), Value::Null, None),
            (json!(-1), Value::Null, None),
            (json!(12.12), Value::Null, None),
            (json!(true), Value::Null, None),
            (json!(false), Value::Null, None),
            (json!("test"), Value::Null, None),
            (json!(""), Value::Null, None),
            (json!({}), Value::Null, None),
            (json!({ "a": 12.12}), Value::Null, None),
            (json!([]), Value::Null, None),
            (json!([0, 1]), Value::Null, None),
            // Bool
            (Value::Null, Value::Bool(false), None),
            (json!(0), Value::Bool(false), Some(Ordering::Equal)),
            (json!(1), Value::Bool(false), Some(Ordering::Greater)),
            (json!(-1), Value::Bool(false), Some(Ordering::Less)),
            (json!(12.12), Value::Bool(false), Some(Ordering::Greater)),
            (json!(true), Value::Bool(false), Some(Ordering::Greater)),
            (json!(false), Value::Bool(false), Some(Ordering::Equal)),
            (json!("test"), Value::Bool(false), Some(Ordering::Greater)),
            (json!(""), Value::Bool(false), Some(Ordering::Greater)),
            (json!({}), Value::Bool(false), Some(Ordering::Greater)),
            (
                json!({ "a": 12.12}),
                Value::Bool(false),
                Some(Ordering::Greater),
            ),
            (json!([]), Value::Bool(false), Some(Ordering::Greater)),
            (json!([0, 1]), Value::Bool(false), Some(Ordering::Greater)),
        ];
        for (ref a, ref b, expected) in tests {
            let a: JsonValue = a.into();
            let b: JsonValue = b.into();
            let result = PartialOrd::partial_cmp(&a, &b);
            assert_eq!(
                result, expected,
                "expected {:?}.partial_cmp({:?}) to be {:?}",
                a, b, expected
            );
        }
    }
}
