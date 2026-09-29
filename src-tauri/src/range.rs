use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};

/// 월간 격자(일요일 시작, 42칸)의 [시작일, 끝일) 범위.
pub fn grid_range(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let start = first - Duration::days(first.weekday().num_days_from_sunday() as i64);
    Some((start, start + Duration::days(42)))
}

/// 로컬 시간대 자정을 RFC3339 문자열로.
pub fn to_rfc3339_local(date: NaiveDate) -> String {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .unwrap_or_else(|| Local.from_utc_datetime(&naive))
        .to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn september_2026_starts_on_previous_sunday() {
        assert_eq!(grid_range(2026, 9), Some((d(2026, 8, 30), d(2026, 10, 11))));
    }

    #[test]
    fn month_starting_on_sunday_starts_same_day() {
        assert_eq!(grid_range(2026, 2), Some((d(2026, 2, 1), d(2026, 3, 15))));
    }

    #[test]
    fn invalid_month_is_none() {
        assert_eq!(grid_range(2026, 0), None);
        assert_eq!(grid_range(2026, 13), None);
    }

    #[test]
    fn rfc3339_is_local_midnight() {
        let s = to_rfc3339_local(d(2026, 9, 1));
        assert!(s.starts_with("2026-09-01T00:00:00"), "{s}");
    }
}
