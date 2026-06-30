use std::fmt::Arguments;

use chrono::{DateTime, SecondsFormat, Utc};

fn format_line(at: DateTime<Utc>, message: Arguments<'_>) -> String {
    format!(
        "{} {message}",
        at.to_rfc3339_opts(SecondsFormat::Millis, true)
    )
}

pub(crate) fn write(message: Arguments<'_>) {
    eprintln!("{}", format_line(Utc::now(), message));
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn formats_utc_timestamp_with_millisecond_precision() {
        let fixed = Utc
            .with_ymd_and_hms(2026, 6, 30, 12, 34, 56)
            .single()
            .unwrap()
            + chrono::Duration::milliseconds(789);

        assert_eq!(
            format_line(fixed, format_args!("[oddsportal] starting collection pass")),
            "2026-06-30T12:34:56.789Z [oddsportal] starting collection pass"
        );
    }
}
