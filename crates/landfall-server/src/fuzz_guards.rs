//! Deterministic guards used by malformed/high-cardinality fuzz corpora.

use serde_json::Value;

pub const MAX_JSON_DEPTH: usize = 32;
pub const MAX_ARRAY_ITEMS: usize = 1_000;
pub const MAX_OBJECT_KEYS: usize = 1_000;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ShapeError {
    TooDeep,
    TooManyArrayItems,
    TooManyObjectKeys,
}

/// Validates structural bounds before schema/domain processing begins.
pub fn validate_shape(value: &Value) -> Result<(), ShapeError> {
    walk(value, 0)
}

fn walk(value: &Value, depth: usize) -> Result<(), ShapeError> {
    if depth > MAX_JSON_DEPTH {
        return Err(ShapeError::TooDeep);
    }
    match value {
        Value::Array(items) => {
            if items.len() > MAX_ARRAY_ITEMS {
                return Err(ShapeError::TooManyArrayItems);
            }
            items.iter().try_for_each(|item| walk(item, depth + 1))
        }
        Value::Object(object) => {
            if object.len() > MAX_OBJECT_KEYS {
                return Err(ShapeError::TooManyObjectKeys);
            }
            object.values().try_for_each(|item| walk(item, depth + 1))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corpus_rejects_malformed_shapes_without_panicking() {
        let mut deep = Value::Null;
        for _ in 0..MAX_JSON_DEPTH + 2 {
            deep = Value::Array(vec![deep]);
        }
        assert_eq!(validate_shape(&deep), Err(ShapeError::TooDeep));
        assert_eq!(
            validate_shape(&Value::Array(vec![Value::Null; MAX_ARRAY_ITEMS + 1])),
            Err(ShapeError::TooManyArrayItems)
        );
        let object = (0..MAX_OBJECT_KEYS + 1)
            .map(|i| (format!("label-{i}"), Value::Null))
            .collect();
        assert_eq!(
            validate_shape(&Value::Object(object)),
            Err(ShapeError::TooManyObjectKeys)
        );
    }
}
