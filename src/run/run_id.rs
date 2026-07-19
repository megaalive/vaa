use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(String);

impl RunId {
    #[must_use]
    pub fn generate() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch");
        let secs = now.as_secs();

        let ts = format_unix_utc(secs);
        let random_suffix = format!("{:04x}", (now.subsec_nanos() & 0xFFFF) as u16);
        Self(format!("{ts}-{random_suffix}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn dir_name(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RunId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn format_unix_utc(unix_secs: u64) -> String {
    let days = unix_secs / 86400;
    let remaining = unix_secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}{m:02}{d:02}T{hours:02}{minutes:02}{seconds:02}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_generates_unique_values() {
        let a = RunId::generate();
        let b = RunId::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn run_id_format_matches_expected_pattern() {
        let id = RunId::generate();
        let s = id.as_str();
        assert!(s.len() >= 20, "run id too short: {s}");
        assert!(s.contains('T'), "run id missing timestamp separator: {s}");
        assert!(
            s.ends_with('Z') || s[s.len() - 5..].contains('-'),
            "run id missing suffix: {s}"
        );
    }

    #[test]
    fn run_id_round_trips_through_display() {
        let id = RunId::generate();
        let displayed = format!("{id}");
        assert_eq!(id.as_str(), displayed);
    }

    #[test]
    fn civil_from_days_known_date() {
        let (y, m, d) = civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn format_unix_utc_epoch() {
        let s = format_unix_utc(0);
        assert_eq!(s, "19700101T000000Z");
    }

    #[test]
    fn format_unix_utc_known() {
        let s = format_unix_utc(1_710_806_400);
        assert_eq!(s, "20240319T000000Z");
    }
}
