#[derive(Debug, PartialEq)]
pub enum ParseIntegerError {
    NoDigits,
    Overflow,
}

pub fn parse_integer(input: &str) -> Result<(i64, &str), ParseIntegerError> {
    let mut chars = input.char_indices().peekable();
    let mut result: i64 = 0;
    let mut negative = false;
    let mut has_digits = false;

    // Check for optional sign
    if let Some((_, c)) = chars.peek().copied() {
        if c == '+' || c == '-' {
            negative = c == '-';
            chars.next(); // Consume the sign
        }
    }

    let mut last_idx = 0;

    while let Some((idx, c)) = chars.peek().copied() {
        if c.is_ascii_digit() {
            has_digits = true;
            let digit = c.to_digit(10).unwrap() as i64;
            result = result * 10 + digit;
            // Check for overflow
            if result > i32::MAX as i64 + negative as i64 {
                return Err(ParseIntegerError::Overflow);
            }
            last_idx = idx + c.len_utf8();
            chars.next(); // Consume the digit
        } else {
            break;
        }
    }

    if !has_digits {
        return Err(ParseIntegerError::NoDigits);
    }

    let result = if negative { -result } else { result };

    Ok((result, &input[last_idx..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_integer() {
        let input = "12345abc";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, 12345);
        assert_eq!(remaining, "abc");
    }

    #[test]
    fn test_negative_integer() {
        let input = "-6789xyz";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, -6789);
        assert_eq!(remaining, "xyz");
    }

    #[test]
    fn test_integer_with_plus_sign() {
        let input = "+42 remaining";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, 42);
        assert_eq!(remaining, " remaining");
    }

    #[test]
    fn test_no_digits() {
        let input = "abc123";
        let result = parse_integer(input);
        assert_eq!(result, Err(ParseIntegerError::NoDigits));
    }

    #[test]
    fn test_overflow() {
        let input = "2147483648"; // i32::MAX + 1
        let result = parse_integer(input);
        assert_eq!(result, Err(ParseIntegerError::Overflow));
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let result = parse_integer(input);
        assert_eq!(result, Err(ParseIntegerError::NoDigits));
    }

    #[test]
    fn test_only_sign() {
        let input = "+";
        let result = parse_integer(input);
        assert_eq!(result, Err(ParseIntegerError::NoDigits));
    }

    #[test]
    fn test_invalid_character() {
        let input = "12a34";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, 12);
        assert_eq!(remaining, "a34");
    }

    #[test]
    fn test_zero() {
        let input = "0remaining";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, 0);
        assert_eq!(remaining, "remaining");
    }

    #[test]
    fn test_negative_zero() {
        let input = "-0abc";
        let (number, remaining) = parse_integer(input).unwrap();
        assert_eq!(number, 0);
        assert_eq!(remaining, "abc");
    }
}
