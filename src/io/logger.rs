use anyhow::Result;
use chrono::{self, Local};
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use strip_ansi_escapes::Writer as StripWriter;

#[derive(Default)]
pub struct Logger {
    file: Option<BufWriter<StripWriter<File>>>,
}

impl Logger {
    pub fn start_logging(&mut self, host: &str) -> Result<()> {
        if self.file.is_none() {
            let path = crate::DATA_DIR.clone().join("logs").join(host);
            let _ = std::fs::create_dir(&path);

            let logfile = path.join(format!("{}.log", Local::now().format("%Y%m%d.%H:%M:%S")));
            self.file = Some(BufWriter::new(StripWriter::new(File::create(logfile)?)));
        }
        Ok(())
    }

    pub fn log_line(&mut self, line: &str) -> Result<()> {
        if let Some(mut writer) = self.file.take() {
            writer.write_all(line.as_bytes())?;
            if !line.ends_with('\n') {
                writer.write_all(b"\n")?;
            }
            self.file = Some(writer);
        }
        Ok(())
    }

    pub fn stop_logging(&mut self) -> Result<()> {
        if let Some(mut writer) = self.file.take() {
            writer.flush()?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn is_logging(&self) -> bool {
        self.file.is_some()
    }
}

#[cfg(test)]
mod logger_tests {

    use super::Logger;

    #[test]
    fn test_logger() {
        let mut logger = Logger::default();
        assert!(!logger.is_logging());
        logger.start_logging("hostname").unwrap();
        assert!(logger.is_logging());
        logger.stop_logging().unwrap();
        assert!(!logger.is_logging());
    }
}
