// What to put on the card, from GitHub: the latest TortOS release, the
// Brick's zip or the Pixel's image out of it, checked against the release's
// SHA256SUMS before anything is written. And whether a newer installer is out.
//
// GitHub's "latest" never names a draft or a pre-release, so a beta stays
// out of reach until it is published as a release.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::writer::Progress;

const TORTOS: &str = "ericreinsmidt/TortOS";
const INSTALLER: &str = "ericreinsmidt/TortOS-Installer";
pub const INSTALLER_RELEASES: &str = "https://github.com/ericreinsmidt/TortOS-Installer/releases/latest";
const SUMS: &str = "SHA256SUMS";

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    browser_download_url: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Release {
    tag_name: String,
    assets: Vec<Asset>,
    #[serde(default)]
    html_url: String,
}

impl Release {
    /// "1.3.0", from the tag "v1.3.0"
    pub fn version(&self) -> &str {
        self.tag_name.trim_start_matches('v')
    }

    /// The Brick's zip, TortOS-v1.3.0.zip, or the Pixel's image,
    /// TortOS-v1.3.0-pixel2.img.xz
    pub fn asset(&self, brick: bool) -> Result<&Asset, String> {
        let want = if brick {
            format!("TortOS-v{}.zip", self.version())
        } else {
            format!("TortOS-v{}-pixel2.img.xz", self.version())
        };
        self.assets
            .iter()
            .find(|a| a.name == want)
            .ok_or_else(|| format!("TortOS {} has no {want} to download.", self.version()))
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(concat!("tortos-installer/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .build()
        .into()
}

fn get(url: &str) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    agent().get(url).header("Accept", "application/vnd.github+json").call()
}

fn latest_of(repo: &str) -> Result<Release, ureq::Error> {
    let mut response = get(&format!("https://api.github.com/repos/{repo}/releases/latest"))?;
    let text = response.body_mut().read_to_string()?;
    serde_json::from_str(&text).map_err(|e| ureq::Error::Io(std::io::Error::other(e)))
}

fn unreachable(e: ureq::Error) -> String {
    match e {
        ureq::Error::StatusCode(403) | ureq::Error::StatusCode(429) => {
            "GitHub is asking for a break: too many checks from this network for now. Try again in a while.".into()
        }
        ureq::Error::StatusCode(code) => format!("GitHub answered {code}. Try again in a while."),
        e => format!("Couldn't reach GitHub, where TortOS is downloaded from: {e}"),
    }
}

/// TortOS's latest release.
pub fn latest() -> Result<Release, String> {
    latest_of(TORTOS).map_err(unreachable)
}

/// A newer installer's version, if one is out. Says nothing when GitHub
/// can't be asked: this is only ever a notice.
pub fn newer_installer() -> Option<String> {
    let release = latest_of(INSTALLER).ok()?;
    // Only a release of this repository counts, by its page's address, as
    // GitHub answers a renamed repository's old name too. Its names ignore
    // case, so the comparison does as well
    let page = release.html_url.to_ascii_lowercase();
    if !page.starts_with(&format!("https://github.com/{}/", INSTALLER.to_ascii_lowercase())) {
        return None;
    }
    let theirs = release.version().to_string();
    newer(&theirs, env!("CARGO_PKG_VERSION")).then_some(theirs)
}

/// Whether version `a` is after `b`: numbers compared part by part, so 1.10
/// is after 1.9
fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// The checksum SHA256SUMS gives for `name`
fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (sum, file) = line.split_once(char::is_whitespace)?;
        // "*name" is sha256sum's binary mode
        (file.trim().trim_start_matches('*') == name).then(|| sum.to_ascii_lowercase())
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Downloads the asset into `dir` and checks it. The file is the caller's to
/// remove when it is done with it.
pub fn download(
    release: &Release,
    asset: &Asset,
    dir: &Path,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<PathBuf, String> {
    let sums_asset = release
        .assets
        .iter()
        .find(|a| a.name == SUMS)
        .ok_or_else(|| format!("TortOS {} has no checksums to check the download against.", release.version()))?;
    let sums = get(&sums_asset.browser_download_url)
        .and_then(|mut r| r.body_mut().read_to_string())
        .map_err(unreachable)?;
    let want = sum_for(&sums, &asset.name)
        .ok_or_else(|| format!("TortOS {}'s checksums don't name {}.", release.version(), asset.name))?;

    std::fs::create_dir_all(dir).map_err(|e| format!("Making {}: {e}", dir.display()))?;
    let path = dir.join(&asset.name);
    let result = (|| {
        let response = get(&asset.browser_download_url).map_err(unreachable)?;
        let mut body = response.into_body().into_reader();
        let mut file = std::fs::File::create(&path).map_err(|e| format!("Saving the download: {e}"))?;
        let mut hash = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        let mut done = 0u64;
        loop {
            if cancelled() {
                return Err("cancelled".to_string());
            }
            let n = body.read(&mut buf).map_err(|e| format!("Downloading {}: {e}", asset.name))?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| format!("Saving the download: {e}"))?;
            done += n as u64;
            progress(Progress::Downloading { done, total: asset.size.max(done) });
        }
        file.sync_all().map_err(|e| format!("Saving the download: {e}"))?;
        if done != asset.size {
            return Err(format!("The download ended early, at {done} of {} bytes. Try again.", asset.size));
        }
        if hex(&hash.finalize()) != want {
            return Err(format!("{} didn't match its checksum, so it wasn't used. Try again.", asset.name));
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(path),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, names: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            assets: names
                .iter()
                .map(|n| Asset { name: n.to_string(), size: 1, browser_download_url: String::new() })
                .collect(),
            html_url: String::new(),
        }
    }

    #[test]
    fn picks_each_devices_file() {
        let r = release(
            "v1.3.0",
            &["SHA256SUMS", "TortOS-v1.3.0-pixel2-sources.tar", "TortOS-v1.3.0-pixel2.img.xz", "TortOS-v1.3.0.zip"],
        );
        assert_eq!(r.version(), "1.3.0");
        assert_eq!(r.asset(true).unwrap().name, "TortOS-v1.3.0.zip");
        assert_eq!(r.asset(false).unwrap().name, "TortOS-v1.3.0-pixel2.img.xz");
        assert!(release("v1.0.1", &["TortOS-v1.0.1.zip"]).asset(false).is_err());
    }

    #[test]
    fn compares_versions() {
        assert!(newer("1.10.0", "1.9.0"));
        assert!(newer("1.0.1", "1.0"));
        assert!(!newer("1.0.0", "1.0.0"));
        assert!(!newer("0.9", "1.0.0"));
    }

    #[test]
    fn reads_the_checksums() {
        let sums = "007a44e2  TortOS-v1.3.0.zip\n084b6541  TortOS-v1.3.0-pixel2.img.xz\nABCDEF *other.bin\n";
        assert_eq!(sum_for(sums, "TortOS-v1.3.0.zip").as_deref(), Some("007a44e2"));
        assert_eq!(sum_for(sums, "TortOS-v1.3.0-pixel2.img.xz").as_deref(), Some("084b6541"));
        assert_eq!(sum_for(sums, "other.bin").as_deref(), Some("abcdef"));
        assert_eq!(sum_for(sums, "TortOS-v1.3.0"), None);
    }

    /// The real thing, over the network: TortOS's latest release, both files
    /// downloaded and checked
    #[test]
    #[ignore]
    fn downloads_the_latest() {
        let r = latest().unwrap();
        let dir = std::env::temp_dir().join("tortos-installer-test");
        for brick in [true, false] {
            let asset = r.asset(brick).unwrap();
            let started = std::time::Instant::now();
            let path = download(&r, asset, &dir, &mut |_| {}, &|| false).unwrap();
            println!("{} {} bytes in {:?}", asset.name, asset.size, started.elapsed());
            std::fs::remove_file(path).unwrap();
        }
        println!("newer installer: {:?}", newer_installer());
    }
}
