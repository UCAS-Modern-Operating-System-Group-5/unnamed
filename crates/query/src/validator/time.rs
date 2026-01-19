use super::{Span, ValidationError, ValidationErrorKind, ValidationResult};
use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};

const ABSOLUTE_DATE_FORMAT_LEN: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct TimeRange {
    /// Minimum timestamp (inclusive), None means no lower bound
    pub min: Option<u64>,
    /// Maximum timestamp (inclusive), None means no upper bound
    pub max: Option<u64>,
}

impl TimeRange {
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

    pub fn contains(&self, value: u64) -> bool {
        let above_min = self.min.map_or(true, |min| value >= min);
        let below_max = self.max.map_or(true, |max| value <= max);
        above_min && below_max
    }
}

/// Validate a time specification with optional operators.
///
/// Supported formats:
/// - `>1d` - more recent than 1 day ago
/// - `<1d` - older than 1 day ago  
/// - `>=2024-01-15` - on or after date
/// - `<=2024-01-15` - on or before date
/// - `1d..1w` - between 1 day and 1 week ago
/// - `2024-01-01..2024-12-31` - date range
/// - `..1w` - up to 1 week ago (no lower bound)
/// - `1d..` - from 1 day ago onwards (no upper bound)
pub fn validate_time(value: String, span: Span) -> ValidationResult<TimeRange> {
    let value = value.trim();
    
    if value.is_empty() {
        return Err(ValidationError::new(span, ValidationErrorKind::EmptyValue));
    }
    
    // Check for range syntax: "1d..1w" or "2024-01-01..2024-01-31"
    if let Some((left, right)) = value.split_once("..") {
        let min = if left.trim().is_empty() {
            None
        } else {
            Some(parse_time_value(left.trim(), span)?)
        };

        let max = if right.trim().is_empty() {
            None
        } else {
            Some(parse_time_value(right.trim(), span)?)
        };

        if let (Some(min_val), Some(max_val)) = (min, max) {
            if min_val > max_val {
                return Err(ValidationError::new(
                    span,
                    ValidationErrorKind::InvalidRange {
                        reason: "minimum time is after maximum time".to_string(),
                    },
                ));
            }
        }
        return Ok(TimeRange { min, max });
    }

    if let Some(rest) = value.strip_prefix(">=") {
        let ts = parse_time_value(rest.trim(), span)?;
        return Ok(TimeRange::at_least(ts));
    }

    if let Some(rest) = value.strip_prefix("<=") {
        let ts = parse_time_value(rest.trim(), span)?;
        return Ok(TimeRange::at_most(ts));
    }

    if let Some(rest) = value.strip_prefix('>') {
        let ts = parse_time_value(rest.trim(), span)?;
        return Ok(TimeRange::at_least(ts.saturating_add(1)));
    }

    if let Some(rest) = value.strip_prefix('<') {
        let ts = parse_time_value(rest.trim(), span)?;
        return Ok(TimeRange::at_most(ts.saturating_sub(1)));
    }

    if let Some(rest) = value.strip_prefix('=') {
        let ts = parse_time_value(rest.trim(), span)?;
        return Ok(TimeRange::between(ts, ts));
    }

    // Plain value
    // for dates, match the entire day; for timestamps, exact match
    let ts = parse_time_value(value, span)?;
    if value.len() <= ABSOLUTE_DATE_FORMAT_LEN {
        return Ok(TimeRange::between(ts, ts.saturating_add(86399)));
    }
    Ok(TimeRange::between(ts, ts))
}

fn parse_time_value(s: &str, span: Span) -> ValidationResult<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ValidationError::new(
            span,
            ValidationErrorKind::InvalidTimeSpec {
                value: s.to_string(),
                reason: "empty time specification".to_string(),
            },
        ));
    }

    // Try as unix timestamp first (plain number)
    if let Ok(ts) = s.parse::<u64>() {
        return Ok(ts);
    }

    // Try as relative time (e.g., "1d", "2h", "30min")
    if let Some(ts) = parse_relative_time(s) {
        return Ok(ts);
    }

    // Try as absolute date/time
    if let Some(ts) = parse_absolute_time(s) {
        return Ok(ts);
    }

    Err(ValidationError::new(
        span,
        ValidationErrorKind::InvalidTimeSpec {
            value: s.to_string(),
            reason: "unrecognized time format. Expected: relative (1d, 2h, 1w), \
                     absolute (2024-01-15), or unix timestamp"
                .to_string(),
        },
    ))
}

/// Parse a relative time string like "1d", "2h", "30min" into a Unix timestamp.
/// The result is `now - duration`, representing a point in the past.
fn parse_relative_time(s: &str) -> Option<u64> {
    let s = s.trim();

    // Find where digits end
    let digit_end = s.chars().position(|c| !c.is_ascii_digit())?;
    if digit_end == 0 || digit_end == s.len() {
        return None;
    }

    let num: u64 = s[..digit_end].parse().ok()?;
    let unit = s[digit_end..].trim();
    let seconds = match unit.to_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => num,
        "m" | "min" | "mins" | "minute" | "minutes" => num.checked_mul(60)?,
        "h" | "hr" | "hrs" | "hour" | "hours" => num.checked_mul(3600)?,
        "d" | "day" | "days" => num.checked_mul(86400)?,
        "w" | "wk" | "wks" | "week" | "weeks" => num.checked_mul(604800)?,
        "mo" | "mon" | "month" | "months" => num.checked_mul(2592000)?, // 30 days
        "y" | "yr" | "yrs" | "year" | "years" => num.checked_mul(31536000)?, // 365 days
        _ => return None,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    Some(now.saturating_sub(seconds))
}

/// Parse an absolute date/time string into a Unix timestamp
fn parse_absolute_time(s: &str) -> Option<u64> {
    let s = s.trim();

    let date_formats = ["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d"];

    let time_formats = ["%H:%M:%S", "%H:%M"];

    if s.len() > ABSOLUTE_DATE_FORMAT_LEN {
        // Datetime
        for date_fmt in date_formats {
            for time_fmt in time_formats {
                let fmts = [
                    format!("{}T{}", date_fmt, time_fmt),
                    format!("{} {}", date_fmt, time_fmt),
                ];
                for fmt in fmts {
                    if let Ok(dt) = NaiveDateTime::parse_from_str(s, &fmt) {
                        if let Some(local) = Local.from_local_datetime(&dt).single() {
                            return Some(local.timestamp() as u64);
                        }
                    }
                }
            }
        }
    } else {
        // Date only
        for fmt in &date_formats {
            if let Ok(date) = NaiveDate::parse_from_str(s, fmt) {
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    if let Some(local) = Local.from_local_datetime(&dt).single() {
                        return Some(local.timestamp() as u64);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[fixture]
    fn test_span() -> Span {
        Span {
            start: 0,
            end: 0,
            context: (),
        }
    }

    // ==================== parse_relative_time ====================

    #[rstest]
    #[case("1s")]
    #[case("30min")]
    #[case("2h")]
    #[case("7d")]
    #[case("2w")]
    #[case("1mo")]
    #[case("1y")]
    fn test_parse_relative_time_valid(#[case] input: &str) {
        assert!(
            parse_relative_time(input).is_some(),
            "Expected '{}' to parse as valid relative time",
            input
        );
    }

    #[rstest]
    #[case("abc")]
    #[case("1")]
    #[case("d")]
    #[case("")]
    #[case("1x")]
    fn test_parse_relative_time_invalid(#[case] input: &str) {
        assert!(
            parse_relative_time(input).is_none(),
            "Expected '{}' to be invalid relative time",
            input
        );
    }

    // ==================== parse_absolute_time ====================

    #[rstest]
    #[case("2024-01-15")]
    #[case("2024/01/15")]
    #[case("2024-01-15T10:30:00")]
    #[case("2024-01-15 10:30:00")]
    fn test_parse_absolute_time_valid(#[case] input: &str) {
        assert!(
            parse_absolute_time(input).is_some(),
            "Expected '{}' to parse as valid absolute time",
            input
        );
    }

    #[rstest]
    #[case("invalid")]
    #[case("2024-13-01")]
    #[case("not-a-date")]
    fn test_parse_absolute_time_invalid(#[case] input: &str) {
        assert!(
            parse_absolute_time(input).is_none(),
            "Expected '{}' to be invalid absolute time",
            input
        );
    }

    // ==================== validate_time ====================

    #[rstest]
    #[case(">1d")]
    #[case(">=1d")]
    #[case("<1w")]
    #[case("<=2024-01-15")]
    #[case("=1704067200")]
    fn test_validate_time_operators(#[case] input: String, test_span: Span) {
        assert!(
            validate_time(input.clone(), test_span).is_ok(),
            "Expected '{}' to be a valid time expression",
            input
        );
    }

    #[rstest]
    fn test_validate_time_range(test_span: Span) {
        let result = validate_time("2024-01-01..2024-12-31".into(), test_span);
        assert!(result.is_ok());

        let range = result.unwrap();
        assert!(range.min.is_some(), "Expected range to have min value");
        assert!(range.max.is_some(), "Expected range to have max value");
    }

    #[rstest]
    fn test_single_date_range(test_span: Span) {
        let result = validate_time("2024-01-01".into(), test_span);
        assert!(result.is_ok());

        let range = result.unwrap();
        assert!(
            range
                .max
                .is_some_and(|max| range.min.is_some_and(|min| max - min == 86399))
        )
    }

    // ==================== Additional edge cases ====================

    #[rstest]
    #[case("2024-01-01..", true, false)]
    #[case("..2024-12-31", false, true)]
    fn test_validate_time_open_ranges(
        #[case] input: String,
        #[case] has_min: bool,
        #[case] has_max: bool,
        test_span: Span,
    ) {
        let result = validate_time(input.clone(), test_span);
        assert!(result.is_ok(), "Expected '{}' to be valid", input);

        let range = result.unwrap();
        assert_eq!(range.min.is_some(), has_min);
        assert_eq!(range.max.is_some(), has_max);
    }
}
