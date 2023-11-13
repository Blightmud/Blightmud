use crate::event::Event;
use crate::VERSION;
use std::cmp::Ordering;
use std::{sync::mpsc::Sender, thread};

#[cfg(test)]
use mockall::automock;

use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header;
use serde::Deserialize;

/// URL for fetching JSON description of the latest release. See the GitHub API docs[^1] for more
/// information.
///
/// [^1]: <https://docs.github.com/en/rest/releases/releases?apiVersion=2022-11-28#get-the-latest-release>
const BLIGHTMUD_RELEASES_API_URL: &str =
    "https://api.github.com/repos/blightmud/blightmud/releases/latest";

#[derive(Deserialize)]
struct LatestVersionInfo {
    #[serde(rename = "name")]
    version: String,
    #[serde(rename = "html_url")]
    url: String,
}

#[cfg_attr(test, automock)]
trait FetchVersionInformation {
    fn fetch(&self) -> Option<LatestVersionInfo>;
}

struct Fetcher {
    client: Client,
}

impl Fetcher {
    fn new() -> Self {
        let client = ClientBuilder::new()
            // GitHub requires a USER_AGENT header or will return HTTP 403 Forbidden.
            .default_headers(header::HeaderMap::from_iter(vec![(
                header::USER_AGENT,
                // safety: only errors on non-printable characters.
                header::HeaderValue::from_str(&format!("Blightmud/{}", VERSION)).unwrap(),
            )]))
            .build()
            // safety: errors if TLS backend cannot be initialized, or the resolver cannot load
            // the system configuration.
            .expect("failed to initialize reqwest client");
        Self { client }
    }
}

impl FetchVersionInformation for Fetcher {
    fn fetch(&self) -> Option<LatestVersionInfo> {
        // make a best-effort GET request to find the latest Blightmud release information from
        // the GitHub API. If errors are encountered, return None.
        self.client
            // safety: only errors on URL parse and we use a known good URL at all times.
            .execute(self.client.get(BLIGHTMUD_RELEASES_API_URL).build().unwrap())
            .ok()
            .and_then(|resp| resp.json::<LatestVersionInfo>().ok())
    }
}

fn run(writer: Sender<Event>, current: impl AsRef<str>, fetcher: &dyn FetchVersionInformation) {
    // If we can fetch new version information ...
    if let Some(latest) = fetcher.fetch() {
        // And the latest version is greater than the current ...
        let current = current.as_ref();
        if let Ordering::Greater = latest.version.as_str().cmp(current) {
            // Write information about how to update.
            let (new, url) = (latest.version, latest.url);
            for msg in [
                format!(
                    "There is a newer version of Blightmud available. (current: {current}, new: {new})",
                ),
                format!("Visit {url} to upgrade to latest version"),
            ] {
                writer.send(Event::Info(msg)).unwrap();
            }
        }
    }
}

/// check whether the current version is the latest available. If there is a new version available,
/// write [Event::Info] messages describing it to the provided [Sender].
pub fn check_latest_version(writer: Sender<Event>) {
    thread::Builder::new()
        .name("check-version-thread".to_string())
        .spawn(move || {
            run(writer, format!("v{VERSION}"), &Fetcher::new());
        })
        .ok();
}

#[cfg(test)]
mod test_version_diff {
    use crate::event::Event;
    use crate::net::check_version::{run, LatestVersionInfo, MockFetchVersionInformation};

    use std::sync::mpsc::{channel, Receiver, Sender};

    #[test]
    fn test_check() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| {
            Some(LatestVersionInfo {
                version: "v10.0.0".to_string(),
                url: "https://example.com".to_string(),
            })
        });

        run(writer, "v1.0.0", &fetcher);
        assert_eq!(
            reader.try_recv().unwrap(),
            Event::Info(
                "There is a newer version of Blightmud available. (current: v1.0.0, new: v10.0.0)"
                    .to_string()
            )
        );
        assert_eq!(
            reader.try_recv().unwrap(),
            Event::Info("Visit https://example.com to upgrade to latest version".to_string())
        );
    }

    #[test]
    fn test_no_new_version() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| {
            Some(LatestVersionInfo {
                version: "v1.0.0".to_string(),
                url: "https://example.com".to_string(),
            })
        });

        run(writer, "v1.0.0", &fetcher);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_no_data() {
        let mut fetcher = MockFetchVersionInformation::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();

        fetcher.expect_fetch().times(1).returning(|| None);

        run(writer, "v1.0.0", &fetcher);
        assert!(reader.try_recv().is_err());
    }
}
