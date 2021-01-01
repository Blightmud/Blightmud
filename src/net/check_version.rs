use crate::event::Event;
use crate::VERSION;
use anyhow::Result;
use curl::easy::Easy;
use std::{sync::mpsc::Sender, thread};

#[cfg(test)]
use mockall::automock;

fn diff_versions(old: &str, new: &str) -> bool {
    old < new
}

#[cfg_attr(test, automock)]
trait FetchVersionInformation {
    fn fetch(&self) -> Result<Vec<u8>>;
}

struct Fetcher {}

impl Fetcher {
    fn new() -> Self {
        Self {}
    }
}

impl FetchVersionInformation for Fetcher {
    fn fetch(&self) -> Result<Vec<u8>> {
        let url = "https://api.github.com/repos/blightmud/blightmud/releases/latest";
        let mut response_data = Vec::new();
        let mut easy = Easy::new();
        easy.url(url)?;
        easy.get(true)?;
        easy.useragent("curl")?;

        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    response_data.extend_from_slice(data);
                    Ok(data.len())
                })
                .ok();
            transfer.perform()?;
        }
        Ok(response_data)
    }
}

fn run(writer: Sender<Event>, current: &str, fetcher: &dyn FetchVersionInformation) {
    if let Ok(data) = fetcher.fetch() {
        if let Ok(json) = serde_json::from_slice(&data) {
            let json: serde_json::Value = json;
            let new: String = json["tag_name"].as_str().unwrap_or_default().to_string();
            let url: String = json["html_url"].as_str().unwrap_or_default().to_string();
            if diff_versions(&current, &new) {
                writer
                    .send(Event::Info(format!(
                        "There is a newer version of Blightmud available. (current: {}, new: {})",
                        current, new
                    )))
                    .unwrap();
                writer
                    .send(Event::Info(format!(
                        "Visit {} to upgrade to latest version",
                        url
                    )))
                    .unwrap();
            }
        }
    }
}

pub fn check_latest_version(writer: Sender<Event>) {
    thread::Builder::new()
        .name("check-version-thread".to_string())
        .spawn(move || {
            let fetcher = Fetcher::new();
            let version = format!("v{}", VERSION);
            run(writer, &version, &fetcher);
        })
        .ok();
}

#[cfg(test)]
mod test_version_diff {

    use std::sync::mpsc::{channel, Receiver};

    use anyhow::bail;

    use super::*;

    #[test]
    fn test_check() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| {
            Ok(br#"{"tag_name":"v10.0.0","html_url":"http://example.com"}"#
                .iter()
                .cloned()
                .collect())
        });

        run(writer.clone(), "v1.0.0", &fetcher);
        assert_eq!(
            reader.try_recv().unwrap(),
            Event::Info(format!(
                "There is a newer version of Blightmud available. (current: v1.0.0, new: v10.0.0)",
            ))
        );
        assert_eq!(
            reader.try_recv().unwrap(),
            Event::Info("Visit http://example.com to upgrade to latest version".to_string())
        );
    }

    #[test]
    fn test_no_new_version() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| {
            Ok(br#"{"tag_name":"v1.0.0","html_url":"http://example.com"}"#
                .iter()
                .cloned()
                .collect())
        });

        run(writer.clone(), "v1.0.0", &fetcher);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_bad_data() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher
            .expect_fetch()
            .times(1)
            .returning(|| Ok(br#"{}"#.iter().cloned().collect()));

        run(writer.clone(), "v1.0.0", &fetcher);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_no_data() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| bail!("Error"));

        run(writer.clone(), "v1.0.0", &fetcher);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_version_diff() {
        assert!(diff_versions("v0.1.0", "v0.1.1"));
        assert!(!diff_versions("v0.1.2", "v0.1.1"));
        assert!(diff_versions("v0.1.0", "v0.3.0"));
        assert!(diff_versions("v0.3.0", "v3.0.0"));
    }
}
