use crate::event::Event;
use crate::VERSION;
use curl::easy::Easy;
use std::{sync::mpsc::Sender, thread};

fn diff_versions(old: &str, new: &str) -> bool {
    old < new
}

pub fn check_latest_version(main_writer: Sender<Event>) {
    thread::spawn(move || {
        let url = "https://api.github.com/repos/blightmud/blightmud/releases/latest";
        let mut response_data = Vec::new();

        let mut easy = Easy::new();
        easy.url(url).unwrap();
        easy.get(true).unwrap();
        easy.useragent("curl").unwrap();

        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    response_data.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();
            transfer.perform().ok();
        }

        if let Ok(json) = serde_json::from_slice(&response_data) {
            let json: serde_json::Value = json;
            let new: String = json["tag_name"].as_str().unwrap_or_default().to_string();
            let url: String = json["html_url"].as_str().unwrap_or_default().to_string();
            let old = format!("v{}", VERSION);
            if diff_versions(&old, &new) {
                main_writer
                    .send(Event::Info(format!(
                        "There is a newer version of Blightmud available. (current: {}, new: {})",
                        old, new
                    )))
                    .unwrap();
                main_writer
                    .send(Event::Info(format!(
                        "Visit {} to upgrade to latest version",
                        url
                    )))
                    .unwrap();
            }
        }
    });
}

#[cfg(test)]
mod test_version_diff {

    use super::diff_versions;

    #[test]
    fn test_version_diff() {
        assert!(diff_versions("v0.1.0", "v0.1.1"));
        assert!(!diff_versions("v0.1.2", "v0.1.1"));
        assert!(diff_versions("v0.1.0", "v0.3.0"));
        assert!(diff_versions("v0.3.0", "v3.0.0"));
    }
}
