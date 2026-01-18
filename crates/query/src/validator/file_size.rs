use super::{Span, ValidationError, ValidationErrorKind, ValidationResult};

#[derive(Debug, Clone, PartialEq)]
pub struct SizeRange {
    /// Minimum size in bytes (inclusive)
    pub min: Option<u64>,
    /// Maximum size in bytes (inclusive)
    pub max: Option<u64>,
}

impl SizeRange {
    pub fn at_least(min: u64) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }
    pub fn at_most(max: u64) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }
    pub fn between(min: u64, max: u64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }
    pub fn exactly(size: u64) -> Self {
        Self {
            min: Some(size),
            max: Some(size),
        }
    }
    pub fn contains(&self, value: u64) -> bool {
        let above_min = self.min.map_or(true, |min| value >= min);
        let below_max = self.max.map_or(true, |max| value <= max);
        above_min && below_max
    }
}


pub fn parse_size_value(s: &str, span: Span) -> ValidationResult<u64> {
    let s = s.trim();

    if s.is_empty() {
        return Err(ValidationError::new(span, ValidationErrorKind::EmptyValue));
    }
    
    // Find where the numeric part ends
    let num_end = s
        .chars()
        .position(|c| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    
    if num_end == 0 {
        return Err(ValidationError::new(
            span,
            ValidationErrorKind::InvalidSizeSpec {
                value: s.to_string(),
                reason: "missing numeric value".to_string(),
            },
        ));
    }
    
    let num_str = &s[..num_end];
    let num: f64 = num_str.parse().map_err(|_| {
        ValidationError::new(
            span,
            ValidationErrorKind::InvalidSizeSpec {
                value: s.to_string(),
                reason: format!("invalid number '{}'", num_str),
            },
        )
    })?;
    
    if num < 0.0 {
        return Err(ValidationError::new(
            span,
            ValidationErrorKind::InvalidSizeSpec {
                value: s.to_string(),
                reason: "size cannot be negative".to_string(),
            },
        ));
    }
    
    let unit = s[num_end..].trim();
    let multiplier: u64 = if unit.is_empty() {
        1 // bytes
    } else {
        match unit.to_lowercase().as_str() {
            // Bytes
            "b" | "byte" | "bytes" => 1,
            // Decimal (SI) units
            "k" | "kb" => 1_000,
            "m" | "mb" => 1_000_000,
            "g" | "gb" => 1_000_000_000,
            "t" | "tb" => 1_000_000_000_000,
            // Binary (IEC) units
            "ki" | "kib" => 1_024,
            "mi" | "mib" => 1_048_576,
            "gi" | "gib" => 1_073_741_824,
            "ti" | "tib" => 1_099_511_627_776,
            _ => {
                return Err(ValidationError::new(
                    span,
                    ValidationErrorKind::InvalidSizeSpec {
                        value: s.to_string(),
                        reason: format!(
                            "unknown unit '{}'. Supported: B, KB, MB, GB, TB, KiB, MiB, GiB, TiB",
                            unit
                        ),
                    },
                ))
            }
        }
    };
    
    Ok((num * multiplier as f64).round() as u64)
}


/// Validate a size specification with optional operators.
/// 
/// Supported formats:
/// - `>1MB` - larger than 1MB
/// - `<100KB` - smaller than 100KB
/// - `>=1GiB` - at least 1GiB
/// - `<=500MB` - at most 500MB
/// - `=1024` - exactly 1024 bytes
/// - `1MB..10MB` - between 1MB and 10MB
/// - `..1GB` - up to 1GB
/// - `100MB..` - at least 100MB
pub fn validate_size(value: String, span: Span) -> ValidationResult<SizeRange> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ValidationError::new(span, ValidationErrorKind::EmptyValue));
    }
    // Check for range syntax: "1MB..10MB"
    if let Some((left, right)) = value.split_once("..") {
        let min = if left.trim().is_empty() {
            None
        } else {
            Some(parse_size_value(left.trim(), span)?)
        };
        
        let max = if right.trim().is_empty() {
            None
        } else {
            Some(parse_size_value(right.trim(), span)?)
        };
        
        if let (Some(min_val), Some(max_val)) = (min, max) {
            if min_val > max_val {
                return Err(ValidationError::new(
                    span,
                    ValidationErrorKind::InvalidRange {
                        reason: format!(
                            "minimum size ({}) is greater than maximum size ({})",
                            min_val, max_val
                        ),
                    },
                ));
            }
        }
        return Ok(SizeRange { min, max });
    }
    
    if let Some(rest) = value.strip_prefix(">=") {
        return Ok(SizeRange::at_least(parse_size_value(rest.trim(), span)?));
    }
    
    if let Some(rest) = value.strip_prefix("<=") {
        return Ok(SizeRange::at_most(parse_size_value(rest.trim(), span)?));
    }
    
    if let Some(rest) = value.strip_prefix('>') {
        let size = parse_size_value(rest.trim(), span)?;
        return Ok(SizeRange::at_least(size.saturating_add(1)));
    }
    
    if let Some(rest) = value.strip_prefix('<') {
        let size = parse_size_value(rest.trim(), span)?;
        return Ok(SizeRange::at_most(size.saturating_sub(1)));
    }
    
    if let Some(rest) = value.strip_prefix('=') {
        let size = parse_size_value(rest.trim(), span)?;
        return Ok(SizeRange::exactly(size));
    }
    
    // Exact match
    let size = parse_size_value(value, span)?;
    Ok(SizeRange::exactly(size))
}


#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn test_span() -> Span {
        Span { start: 0, end: 0, context: () }
    }

    #[rstest]
    #[case("1024", 1024)]
    #[case("0", 0)]
    fn test_parse_size_bytes(#[case] input: &str, #[case] expected: u64) {
        assert_eq!(parse_size_value(input, test_span()).unwrap(), expected);
    }

    #[rstest]
    #[case("1KB", 1_000)]
    #[case("1KiB", 1_024)]
    #[case("1MB", 1_000_000)]
    #[case("1MiB", 1_048_576)]
    #[case("1.5MB", 1_500_000)]
    #[case("1 GB", 1_000_000_000)]
    fn test_parse_size_units(#[case] input: &str, #[case] expected: u64) {
        assert_eq!(parse_size_value(input, test_span()).unwrap(), expected);
    }

    #[rstest]
    #[case(">1MB", Some(1_000_001), None)]
    #[case(">=1MB", Some(1_000_000), None)]
    #[case("<1KB", None, Some(999))]
    #[case("=1024", Some(1024), Some(1024))]
    fn test_validate_size_operators(
        #[case] input: String,
        #[case] expected_min: Option<u64>,
        #[case] expected_max: Option<u64>,
    ) {
        let result = validate_size(input, test_span()).unwrap();
        assert_eq!(result.min, expected_min);
        assert_eq!(result.max, expected_max);
    }

    #[rstest]
    #[case("1MB..10MB", Some(1_000_000), Some(10_000_000))]
    #[case("..1GB", None, Some(1_000_000_000))]
    #[case("100MB..", Some(100_000_000), None)]
    fn test_validate_size_range(
        #[case] input: String,
        #[case] expected_min: Option<u64>,
        #[case] expected_max: Option<u64>,
    ) {
        let result = validate_size(input, test_span()).unwrap();
        assert_eq!(result.min, expected_min);
        assert_eq!(result.max, expected_max);
    }

    #[rstest]
    #[case("10MB..1MB")]
    #[case("1GB..1MB")]
    fn test_validate_size_invalid_range(#[case] input: String) {
        let result = validate_size(input.into(), test_span());
        assert!(result.is_err());
    }
}
