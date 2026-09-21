//! Minimal ISO 8601 calendar-date (`YYYY-MM-DD`) validation and
//! ordering — no chrono dependency needed for a check this narrow.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u32,
    pub month: u8,
    pub day: u8,
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parses a strict `YYYY-MM-DD` string, validating that the month is
/// 01-12 and the day is a real day of that month in that year
/// (including the February-29 leap-year rule).
pub fn parse(s: &str) -> Option<Date> {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: u32 = s.get(0..4)?.parse().ok()?;
    let month: u8 = s.get(5..7)?.parse().ok()?;
    let day: u8 = s.get(8..10)?.parse().ok()?;

    if !(1..=12).contains(&month) {
        return None;
    }
    if day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some(Date { year, month, day })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn parses_a_valid_date() {
        let d = parse("2024-03-15").unwrap();
        assert_eq!((d.year, d.month, d.day), (2024, 3, 15));
    }

    #[test]
    fn rejects_month_13() {
        assert!(parse("2024-13-01").is_none());
    }

    #[test]
    fn rejects_month_00() {
        assert!(parse("2024-00-01").is_none());
    }

    #[test]
    fn rejects_april_31st() {
        assert!(parse("2024-04-31").is_none());
    }

    #[test]
    fn accepts_february_29_in_a_leap_year() {
        assert!(parse("2024-02-29").is_some());
    }

    #[test]
    fn rejects_february_29_in_a_non_leap_year() {
        assert!(parse("2023-02-29").is_none());
    }

    #[test]
    fn rejects_february_29_in_a_century_non_leap_year() {
        assert!(parse("1900-02-29").is_none()); // divisible by 100 but not 400
    }

    #[test]
    fn accepts_february_29_in_a_400_divisible_year() {
        assert!(parse("2000-02-29").is_some());
    }

    #[test]
    fn rejects_malformed_separators() {
        assert!(parse("2024/03/15").is_none());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(parse("24-3-15").is_none());
    }

    #[test]
    fn dates_order_naturally_by_derived_ord() {
        let earlier = parse("2024-01-01").unwrap();
        let later = parse("2024-06-01").unwrap();
        assert_eq!(earlier.cmp(&later), Ordering::Less);
    }
}
