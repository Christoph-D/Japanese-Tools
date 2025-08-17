pub trait UnicodeByteLimit {
    /// Safely limit the length of a unicode string.
    fn unicode_byte_limit(&self, max_bytes: usize) -> &str;
}

impl UnicodeByteLimit for str {
    fn unicode_byte_limit(&self, max_bytes: usize) -> &str {
        if self.len() <= max_bytes {
            return self;
        }
        for (limit, _) in self.char_indices().rev() {
            if limit <= max_bytes {
                return &self[..limit];
            }
        }
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_byte_limit_ascii() {
        assert_eq!("hello".unicode_byte_limit(5), "hello");
        assert_eq!("hello world".unicode_byte_limit(5), "hello");
        assert_eq!("hello".unicode_byte_limit(10), "hello");
        assert_eq!("hello".unicode_byte_limit(0), "");
    }

    #[test]
    fn test_unicode_byte_limit_japanese() {
        assert_eq!("こんにちは".unicode_byte_limit(15), "こんにちは"); // 5 chars * 3 bytes = 15
        assert_eq!("こんにちは".unicode_byte_limit(12), "こんにち"); // 4 chars * 3 bytes = 12
        assert_eq!("こんにちは".unicode_byte_limit(3), "こ"); // 1 char * 3 bytes = 3
        assert_eq!("こんにちは".unicode_byte_limit(2), ""); // Not enough for 1 char
        for i in 1..=16 {
            "こんにちは".unicode_byte_limit(i); // assert no panics
        }
    }

    #[test]
    fn test_unicode_byte_limit_mixed() {
        assert_eq!("hello こんにちは".unicode_byte_limit(10), "hello こ"); // 6 ASCII + 1 Japanese = 9 bytes
    }

    #[test]
    fn test_unicode_byte_limit_empty_string() {
        assert_eq!("".unicode_byte_limit(10), "");
        assert_eq!("".unicode_byte_limit(0), "");
    }
}
