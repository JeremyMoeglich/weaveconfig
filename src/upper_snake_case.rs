pub fn to_upper_snake_case(input: &str) -> String {
    let mut result = String::new();
    let mut prev_char = '_'; // Initialize as an underscore for easier handling of first character

    for (i, ch) in input.chars().enumerate() {
        if !ch.is_ascii() {
            result.push_str(&format!("U_{:04X}", ch as u32));
            prev_char = '_';
            continue;
        }

        match ch {
            // Handle underscores, hyphens, and spaces as word separators
            '_' | '-' | ' ' => {
                if prev_char != '_' {
                    result.push('_');
                }
                prev_char = '_';
            }
            // Handle uppercase letters, separating them as word boundaries if necessary
            'A'..='Z' => {
                if i > 0 && prev_char != '_' && !prev_char.is_uppercase() {
                    result.push('_');
                }
                result.push(ch);
                prev_char = ch;
            }
            // Handle lowercase letters, converting them to uppercase
            'a'..='z' => {
                if prev_char == '_' {
                    result.push(ch.to_ascii_uppercase());
                } else {
                    result.push(ch.to_ascii_uppercase());
                }
                prev_char = ch;
            }
            // Handle numbers, but avoid separating consecutive digits
            '0'..='9' => {
                if !prev_char.is_digit(10) && prev_char != '_' {
                    result.push('_');
                }
                result.push(ch);
                prev_char = ch;
            }
            _ => {
                // Ignore any other character for now
            }
        }
    }

    // Ensure that there is no trailing underscore
    if result.ends_with('_') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case() {
        assert_eq!(to_upper_snake_case("someCasing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("someCasingWithNumbers123"), "SOME_CASING_WITH_NUMBERS_123");
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(to_upper_snake_case("some_casing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("some_snake_case"), "SOME_SNAKE_CASE");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_upper_snake_case("SomeCasing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("PascalCaseExample"), "PASCAL_CASE_EXAMPLE");
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(to_upper_snake_case("some-casing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("kebab-case-example"), "KEBAB_CASE_EXAMPLE");
    }

    #[test]
    fn test_mixed_case() {
        assert_eq!(to_upper_snake_case("Some_Casing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("SOME_casing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("Some-Casing_WithMixed-Styles"), "SOME_CASING_WITH_MIXED_STYLES");
    }

    #[test]
    fn test_numbers_and_special_characters() {
        assert_eq!(to_upper_snake_case("someCasing123"), "SOME_CASING_123");
        assert_eq!(to_upper_snake_case("some_casing-123"), "SOME_CASING_123");
        assert_eq!(to_upper_snake_case("some-casing!@#123"), "SOME_CASING_123");
    }

    #[test]
    fn test_already_upper_snake_case() {
        assert_eq!(to_upper_snake_case("SOME_CASING"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("ALREADY_UPPER_CASE"), "ALREADY_UPPER_CASE");
    }

    #[test]
    fn test_empty_and_single_character() {
        assert_eq!(to_upper_snake_case(""), "");
        assert_eq!(to_upper_snake_case("a"), "A");
        assert_eq!(to_upper_snake_case("A"), "A");
        assert_eq!(to_upper_snake_case("1"), "1");
    }

    #[test]
    fn test_spaces() {
        assert_eq!(to_upper_snake_case("some casing"), "SOME_CASING");
        assert_eq!(to_upper_snake_case("with multiple spaces"), "WITH_MULTIPLE_SPACES");
        assert_eq!(to_upper_snake_case("  leading and trailing spaces  "), "LEADING_AND_TRAILING_SPACES");
    }

    #[test]
    fn test_non_ascii_characters() {
        assert_eq!(to_upper_snake_case("café"), "CAFU_00E9");
        assert_eq!(to_upper_snake_case("résumé"), "RU_00E9SUMU_00E9");
        assert_eq!(to_upper_snake_case("こんにちは"), "U_3053U_3093U_306BU_3061U_306F");
    }
}