use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

// Constants

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/TripleT3005/FlexPad/releases/latest";
const RELEASE_PAGE_PREFIX: &str = "https://github.com/TripleT3005/FlexPad/releases/";
const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: u64 = 256 * 1024;

// Types

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
}

// Versions

pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let mut next = || {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

pub fn is_release_url(url: &str) -> bool {
    url.strip_prefix(RELEASE_PAGE_PREFIX).is_some_and(|rest| {
        !rest.is_empty() && !rest.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}

pub fn parse_release(json: &str) -> Option<ReleaseInfo> {
    let response: ReleaseResponse = serde_json::from_str(json).ok()?;
    let version = response.tag_name.strip_prefix('v')?;
    parse_version(version)?;
    is_release_url(&response.html_url).then(|| ReleaseInfo {
        version: version.to_string(),
        url: response.html_url,
    })
}

pub fn newer_release(json: &str, current: &str) -> Option<ReleaseInfo> {
    parse_release(json).filter(|release| is_newer(&release.version, current))
}

// Network

pub fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let json = agent
        .get(LATEST_RELEASE_URL)
        .header("User-Agent", format!("FlexPad/{APP_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(newer_release(&json, APP_VERSION))
}

// Browser

pub fn open_in_browser(url: &str) {
    if !is_release_url(url) {
        return;
    }
    let _ = browser_command(url).spawn();
}

#[cfg(target_os = "windows")]
fn browser_command(url: &str) -> Command {
    let mut command = Command::new("explorer");
    command.arg(url);
    command
}

#[cfg(target_os = "macos")]
fn browser_command(url: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(url);
    command
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn browser_command(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_JSON: &str = r#"{"tag_name":"v1.2.0","name":"FlexPad 1.2.0","html_url":"https://github.com/TripleT3005/FlexPad/releases/tag/v1.2.0","assets":[]}"#;

    fn release_json(tag: &str, url: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","html_url":"{url}"}}"#)
    }

    // Versions

    #[test]
    fn parses_three_numeric_parts() {
        assert_eq!(parse_version("0.10.2"), Some((0, 10, 2)));
        for bad in [
            "",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1.2.3-beta.1",
            "v1.2.3",
            "1..3",
            "+1.2.3",
        ] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
    }

    #[test]
    fn compares_numerically() {
        assert!(is_newer("0.10.0", "0.4.1"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
        assert!(!is_newer("garbage", "0.2.0"));
        assert!(!is_newer("0.3.0", "garbage"));
    }

    // Release page

    #[test]
    fn accepts_only_release_urls_of_this_project() {
        assert!(is_release_url(
            "https://github.com/TripleT3005/FlexPad/releases/tag/v0.3.0"
        ));
        assert!(!is_release_url(
            "https://github.com/TripleT3005/FlexPad/releases/"
        ));
        assert!(!is_release_url(
            "https://github.com/TripleT3005/FlexPad/issues/1"
        ));
        assert!(!is_release_url(
            "https://github.com/TripleT3005/FlexPadX/releases/tag/v1"
        ));
        assert!(!is_release_url(
            "http://github.com/TripleT3005/FlexPad/releases/tag/v1"
        ));
        assert!(!is_release_url(
            "https://evil.example/https://github.com/TripleT3005/FlexPad/releases/"
        ));
        assert!(!is_release_url(
            "https://github.com/TripleT3005/FlexPad/releases/a b"
        ));
    }

    // Response

    #[test]
    fn reads_the_latest_release() {
        assert_eq!(
            parse_release(RELEASE_JSON),
            Some(ReleaseInfo {
                version: "1.2.0".to_string(),
                url: "https://github.com/TripleT3005/FlexPad/releases/tag/v1.2.0".to_string(),
            })
        );
    }

    #[test]
    fn rejects_unusable_responses() {
        let page = "https://github.com/TripleT3005/FlexPad/releases/tag/x";
        assert_eq!(parse_release("not json"), None);
        assert_eq!(parse_release("{}"), None);
        assert_eq!(parse_release("[]"), None);
        assert_eq!(parse_release(&release_json("1.2.0", page)), None);
        assert_eq!(parse_release(&release_json("v1.2.0-beta.1", page)), None);
        assert_eq!(
            parse_release(&release_json("v1.2.0", "https://example.com/")),
            None
        );
    }

    #[test]
    fn reports_only_newer_releases() {
        assert!(newer_release(RELEASE_JSON, "1.0.0").is_some());
        assert!(newer_release(RELEASE_JSON, "1.2.0").is_none());
        assert!(newer_release(RELEASE_JSON, "1.10.0").is_none());
    }
}
