#[derive(Debug, PartialEq)]
pub enum ParseSegmentError {
    UnclosedQuote,
    NoSegment,
}

pub fn parse_segment(input: &str) -> Result<(String, &str), ParseSegmentError> {
    let mut chars = input.char_indices().peekable();
    let mut result = String::new();

    // Peek the first character
    if let Some((_, first_char)) = chars.peek().copied() {
        if first_char == '"' || first_char == '\'' {
            // It's a quoted string
            let quote_char = first_char;
            chars.next(); // Consume the opening quote
            let mut escaped = false;

            for (idx, c) in chars {
                if escaped {
                    match c {
                        '\\' => result.push('\\'),
                        '"' if quote_char == '"' => result.push('"'),
                        '\'' if quote_char == '\'' => result.push('\''),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        other => {
                            // If it's an unknown escape sequence, include the backslash and the character
                            result.push('\\');
                            result.push(other);
                        }
                    }
                    escaped = false;
                } else {
                    if c == '\\' {
                        escaped = true;
                    } else if c == quote_char {
                        // Closing quote found
                        let next_idx = idx + c.len_utf8();
                        return Ok((result, &input[next_idx..]));
                    } else {
                        result.push(c);
                    }
                }
            }

            // If we reach here, the quoted string was not closed
            Err(ParseSegmentError::UnclosedQuote)
        } else if first_char.is_alphanumeric() || first_char == '_' || first_char == '-' {
            // It's an alphanumeric, underscore, or hyphen segment
            for (idx, c) in chars {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    result.push(c);
                } else {
                    // Found a non-alphanumeric character, return up to this point
                    let next_idx = idx;
                    return Ok((result, &input[next_idx..]));
                }
            }
            // Reached the end of the input
            Ok((result, ""))
        } else {
            // The segment does not start with a quote, alphanumeric, underscore, or hyphen
            // Return an error as zero characters are parsed
            Err(ParseSegmentError::NoSegment)
        }
    } else {
        // Empty input
        Ok((result, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_whitespace(input: &str) -> &str {
        input.trim_start()
    }

    #[test]
    fn test_alphanumeric() {
        let input = "helloWorld123!@#";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "helloWorld123");
        assert_eq!(remaining, "!@#");
    }

    #[test]
    fn test_alphanumeric_with_underscores_and_hyphens() {
        let input = "hello_world-123!@#";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "hello_world-123");
        assert_eq!(remaining, "!@#");
    }

    #[test]
    fn test_double_quoted() {
        let input = "\"Hello, \\\"World\\\"!\" remaining";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "Hello, \"World\"!");
        assert_eq!(remaining, " remaining");
    }

    #[test]
    fn test_single_quoted() {
        let input = "'It\\'s a test' and more";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "It's a test");
        assert_eq!(remaining, " and more");
    }

    #[test]
    fn test_unclosed_quote() {
        let input = "\"Unclosed string";
        let result = parse_segment(input);
        assert_eq!(result, Err(ParseSegmentError::UnclosedQuote));
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "");
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_non_alphanumeric_start() {
        let input = "!invalid";
        let result = parse_segment(input);
        assert_eq!(result, Err(ParseSegmentError::NoSegment));
    }

    #[test]
    fn test_escape_sequences() {
        let input = "\"Line1\\nLine2\\tTabbed\" end";
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "Line1\nLine2\tTabbed");
        assert_eq!(remaining, " end");
    }

    #[test]
    fn test_mixed_input() {
        let input = "start-1 \"quoted \\\"string\\\"\" middle-2 'another \\'test\\'' end-3";

        // Parse first segment
        let (segment, remaining) = parse_segment(input).unwrap();
        assert_eq!(segment, "start-1");
        // Skip leading whitespace before parsing next segment
        let remaining = skip_whitespace(remaining);
        assert_eq!(
            remaining,
            "\"quoted \\\"string\\\"\" middle-2 'another \\'test\\'' end-3"
        );

        // Parse second segment
        let (segment, remaining) = parse_segment(remaining).unwrap();
        assert_eq!(segment, "quoted \"string\"");
        // Skip leading whitespace before parsing next segment
        let remaining = skip_whitespace(remaining);
        assert_eq!(remaining, "middle-2 'another \\'test\\'' end-3");

        // Parse third segment
        let (segment, remaining) = parse_segment(remaining).unwrap();
        assert_eq!(segment, "middle-2");
        // Skip leading whitespace before parsing next segment
        let remaining = skip_whitespace(remaining);
        assert_eq!(remaining, "'another \\'test\\'' end-3");

        // Parse fourth segment
        let (segment, remaining) = parse_segment(remaining).unwrap();
        assert_eq!(segment, "another 'test'");
        // Skip leading whitespace before parsing next segment
        let remaining = skip_whitespace(remaining);
        assert_eq!(remaining, "end-3");

        // Parse fifth segment
        let (segment, remaining) = parse_segment(remaining).unwrap();
        assert_eq!(segment, "end-3");
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_zero_characters_parsed_error() {
        let inputs = vec![" ", "!", "@#"];

        for input in inputs {
            let result = parse_segment(input);
            assert_eq!(result, Err(ParseSegmentError::NoSegment));
        }
    }

    #[test]
    fn test_valid_segment_starts() {
        let inputs = vec!["_valid_start", "-valid-start", "valid-middle-1"];

        for input in inputs {
            let (segment, remaining) = parse_segment(input).unwrap();
            assert_eq!(segment, input);
            assert_eq!(remaining, "");
        }
    }
}
