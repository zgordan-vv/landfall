use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CursorError {
    #[error("invalid cursor")]
    Invalid,
    #[error("page limit must be between 1 and 1000")]
    InvalidLimit,
}

/// Paginates an already deterministically ordered slice with an opaque offset cursor.
pub fn paginate<T: Clone>(
    items: &[T],
    cursor: Option<&str>,
    limit: usize,
) -> Result<Page<T>, CursorError> {
    if !(1..=1000).contains(&limit) {
        return Err(CursorError::InvalidLimit);
    }
    let start = cursor.map_or(Ok(0), decode_cursor)?;
    if start > items.len() {
        return Err(CursorError::Invalid);
    }
    let end = start.saturating_add(limit).min(items.len());
    let next_cursor = (end < items.len()).then(|| encode_cursor(end));
    Ok(Page {
        items: items[start..end].to_vec(),
        next_cursor,
    })
}

fn encode_cursor(offset: usize) -> String {
    format!("lfc1_{offset:x}")
}
fn decode_cursor(cursor: &str) -> Result<usize, CursorError> {
    let value = cursor.strip_prefix("lfc1_").ok_or(CursorError::Invalid)?;
    usize::from_str_radix(value, 16).map_err(|_| CursorError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_pages_are_bounded_and_repeatable() {
        let source = vec![1, 2, 3, 4, 5];
        let first = paginate(&source, None, 2).expect("page");
        assert_eq!(first.items, vec![1, 2]);
        let second = paginate(&source, first.next_cursor.as_deref(), 2).expect("page");
        assert_eq!(second.items, vec![3, 4]);
        let third = paginate(&source, second.next_cursor.as_deref(), 2).expect("page");
        assert_eq!(third.items, vec![5]);
        assert!(third.next_cursor.is_none());
    }
    #[test]
    fn invalid_cursor_and_limit_are_rejected() {
        assert_eq!(
            paginate::<u8>(&[], Some("bad"), 1),
            Err(CursorError::Invalid)
        );
        assert_eq!(paginate::<u8>(&[], None, 0), Err(CursorError::InvalidLimit));
    }
}
