use clap::{Parser, Args, ValueEnum};
use chrono::prelude::*;
use chrono::{Duration, DurationRound};
use regex::{Regex};
use parse_duration::parse as parse_duration;


#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    format: OutputFormat,

    input: Option<String>,

    #[arg(short, long = "output", group = "read")]
    output_format: Option<ReadableOutputFormat>,
}

#[derive(Debug, Args, Clone)]
#[group(required = true, multiple = false)]
struct OutputFormat {
    #[arg(short, long)]
    epoch: bool,
    #[arg(short, long)]
    millis: bool,
    #[arg(short, long, requires = "read")]
    readable: bool,
}

#[derive(Debug, ValueEnum, Clone)]
enum ReadableOutputFormat {
    UTC,
    Local,
}

fn try_get_relative_dt(input: &str) -> Option<DateTime<Utc>> {
    /* make sure ends with qualifier, extract it out,
    parse the front time unit part and produce time instant based on that
    */
    let qualifier_r = Regex::new(r#".*\s+(ago|later)"#).unwrap();
    let qualifier = qualifier_r.captures(input).and_then(|groups| groups.get(1)).map(|q_group| q_group.as_str());
    if qualifier.is_some() {
        let only_time_unit = input.trim_end_matches("(ago|later)");
        parse_duration(only_time_unit).ok().and_then(|dur| {
            let now = Utc::now();
            if qualifier.unwrap() == "ago" {
                now.checked_sub_signed(Duration::from_std(dur).unwrap())
            } else {
                now.checked_add_signed(Duration::from_std(dur).unwrap())
            }
        })
    } else {
        None
    }
}

fn parse_string_to_local_datetime(date_string: &str) -> Option<DateTime<Local>> {
    NaiveDateTime::parse_from_str(date_string, "%Y-%m-%d %H:%M:%S")
        .ok().and_then(|dt| Local.from_local_datetime(&dt).single())
}

fn try_get_absolute_dt(input: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input).ok()
        .or(
            parse_string_to_local_datetime(input)
                .map(|dt| dt.with_timezone(Local::now().offset()))
        )
        .map(|dt| dt.with_timezone(Utc::now().offset()))
}

fn input_to_time(input: Option<String>) -> Option<DateTime<Utc>> {
    match input {
        None => Some(Utc::now()),
        Some(str) => try_get_relative_dt(&str).or(try_get_absolute_dt(&str))
    }
}

fn produce_time_output(args: Cli) -> String {
    let (show_epoch, show_millis, show_readable) = (args.format.epoch, args.format.millis, args.format.readable);

    let dt = input_to_time(args.input).expect("Invalid input, not able to parse input, input when defined must comply to `rfc 3339`, `YYYY-MM-DD`");

    match (show_epoch, show_millis, show_readable) {
        (true, _, _) => dt.timestamp().to_string(),
        (_, true, _) => dt.timestamp_millis().to_string(),
        (_, _, true) =>
            match args.output_format {
                None => unreachable!(),
                Some(ReadableOutputFormat::UTC) =>
                    dt.with_timezone(Utc::now().offset()).duration_trunc(Duration::milliseconds(100)).expect("Failed to truncate time to millis").to_rfc3339(),
                Some(ReadableOutputFormat::Local) =>
                    dt.with_timezone(Local::now().offset()).duration_trunc(Duration::milliseconds(100)).expect("Failed to truncate time to millis").to_rfc3339()
            }
        _ => unreachable!()
    }
}

fn main() {
    let args = Cli::parse();
    let output = produce_time_output(args);
    println!("{}", output);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_input_epoch() {
        let arg = Cli {
            format: OutputFormat { epoch: true, millis: false, readable: false },
            input: None,
            output_format: None,
        };
        let res = produce_time_output(arg);
        let expected = Utc::now().timestamp_millis();
        assert!(res.parse::<i64>().unwrap() * 1000 <= expected)
    }

    #[test]
    fn test_input_rfc3339_readable() {
        let arg = Cli {
            format: OutputFormat { epoch: false, millis: false, readable: true },
            input: Some(String::from("2022-02-02T01:00:00Z")),
            output_format: Some(ReadableOutputFormat::Local),
        };
        let res = produce_time_output(arg);
        assert_eq!(DateTime::parse_from_rfc3339(res.as_str()).is_ok(), true);
    }
    #[test]
    fn test_input_local_time() {
        let arg = Cli {
            format: OutputFormat { epoch: false, millis: false, readable: true },
            input: Some(String::from("2022-02-02 01:00:00")),
            output_format: Some(ReadableOutputFormat::Local),
        };
        let res = produce_time_output(arg);
        assert_eq!(DateTime::parse_from_rfc3339(res.as_str()).is_ok(), true);
    }

    #[test]
    fn test_input_relative() {
        let arg = Cli {
            format: OutputFormat { epoch: false, millis: false, readable: true },
            input: Some(String::from("2 hours ago")),
            output_format: Some(ReadableOutputFormat::Local),
        };
        let res = produce_time_output(arg);
        assert_eq!(DateTime::parse_from_rfc3339(res.as_str()).is_ok(), true);
    }


}