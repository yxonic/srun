use chrono::{DateTime, Utc};

use crate::Error;

/// Reporting running status and logs
pub trait Reporter {
    fn emit_stdout(&self, line: &str) -> Result<(), Error> {
        let (ts, line) = line.split_once(' ').expect("expect to timestamp");
        let timestamp =
            DateTime::parse_from_rfc3339(ts).expect("expect timestamp to be of RFC3339");
        self.report_stdout(line.trim_end(), timestamp.into())
    }
    fn emit_stderr(&self, line: &str) -> Result<(), Error> {
        let (ts, line) = line.split_once(' ').expect("expect to timestamp");
        let timestamp =
            DateTime::parse_from_rfc3339(ts).expect("expect timestamp to be of RFC3339");
        self.report_stderr(line.trim_end(), timestamp.into())
    }
    fn emit_console(&self, line: &str) -> Result<(), Error> {
        self.report_stdout(line.trim_end(), Utc::now())
    }
    fn report_stdout(&self, line: &str, timestamp: DateTime<Utc>) -> Result<(), Error>;
    fn report_stderr(&self, line: &str, timestamp: DateTime<Utc>) -> Result<(), Error>;
}

pub struct TextReporter;

impl Reporter for TextReporter {
    fn report_stdout(&self, line: &str, _: DateTime<Utc>) -> Result<(), Error> {
        println!("{}", line);
        Ok(())
    }
    fn report_stderr(&self, line: &str, _: DateTime<Utc>) -> Result<(), Error> {
        eprintln!("{}", line);
        Ok(())
    }
}
