use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::{Cache, Channel, Updates};

pub const REPO_OWNER: &str = "knst0";
pub const REPO_NAME: &str = "reveal";
#[cfg(windows)]
pub const BIN_NAME: &str = "reveal.exe";
#[cfg(not(windows))]
pub const BIN_NAME: &str = "reveal";

pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Disabled,
    UpToDate,
    Available { version: String },
    Installed { version: String },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateNotice {
    Available { version: String },
    Installed { version: String },
    Upgraded { from: Option<String>, to: String },
}

fn split_version(v: &str) -> (Vec<u64>, Option<&str>) {
    let v = v.trim_start_matches('v').trim();
    let v = v.split('+').next().unwrap_or_default();
    let (core, pre) = match v.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (v, None),
    };
    let numbers = core.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect();
    (numbers, pre.filter(|p| !p.is_empty()))
}

fn compare_prerelease(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ordering = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

pub fn is_newer(current: &str, candidate: &str) -> bool {
    use std::cmp::Ordering;
    let (a, a_pre) = split_version(current);
    let (b, b_pre) = split_version(candidate);

    let len = a.len().max(b.len());
    for i in 0..len {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if y != x {
            return y > x;
        }
    }

    match (a_pre, b_pre) {
        (None, None) => false,
        (None, Some(_)) => false,
        (Some(_), None) => true,
        (Some(x), Some(y)) => compare_prerelease(x, y) == Ordering::Less,
    }
}

pub fn is_prerelease(version: &str) -> bool {
    version.trim_start_matches('v').contains('-')
}

pub fn accepts(channel: Channel, version: &str) -> bool {
    channel.accepts_prerelease() || !is_prerelease(version)
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn is_due(last_check: Option<u64>, now: u64, interval: Duration) -> bool {
    match last_check {
        None => true,
        Some(last) => now.saturating_sub(last) >= interval.as_secs(),
    }
}

pub fn is_skipped(cache: &Cache, version: &str) -> bool {
    cache.update_skipped_version.as_deref() == Some(version)
}

pub fn should_run_check(settings: &Updates, cache: &Cache) -> bool {
    settings.should_check() && is_due(cache.last_update_check, now_secs(), CHECK_INTERVAL)
}

pub fn record_check(cache: &mut Cache) {
    cache.last_update_check = Some(now_secs());
}

pub fn skip_version(cache: &mut Cache, version: &str) {
    cache.update_skipped_version = Some(version.to_owned());
}

pub fn take_upgrade_notice(cache: &mut Cache) -> Option<UpdateNotice> {
    let current = current_version();
    let pending = cache.update_pending_version.take();
    let previous = cache.last_launched_version.clone();
    cache.last_launched_version = Some(current.to_owned());

    let upgraded = match (&pending, &previous) {
        (Some(pending), _) => !is_newer(current, pending),
        (None, Some(previous)) => is_newer(previous, current),
        (None, None) => false,
    };

    upgraded.then(|| UpdateNotice::Upgraded { from: previous, to: current.to_owned() })
}

pub fn changelog_url(version: &str) -> String {
    let tag = if version.starts_with('v') { version.to_owned() } else { format!("v{version}") };
    format!("https://github.com/{REPO_OWNER}/{REPO_NAME}/releases/tag/{tag}")
}

pub fn pick_upgrade<'a>(
    channel: Channel,
    current: &str,
    versions: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    versions.into_iter().filter(|v| accepts(channel, v) && is_newer(current, v)).max_by(|a, b| {
        if is_newer(a, b) { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
    })
}

#[cfg(not(feature = "updates"))]
pub fn check(_channel: Channel) -> UpdateStatus {
    UpdateStatus::Disabled
}

#[cfg(not(feature = "updates"))]
pub fn install(_channel: Channel) -> UpdateStatus {
    UpdateStatus::Disabled
}

#[cfg(feature = "updates")]
pub fn check(channel: Channel) -> UpdateStatus {
    match latest_version(channel) {
        Ok(Some(version)) => UpdateStatus::Available { version },
        Ok(None) => UpdateStatus::UpToDate,
        Err(error) => UpdateStatus::Failed { error },
    }
}

#[cfg(feature = "updates")]
fn latest_version(channel: Channel) -> Result<Option<String>, String> {
    let list = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .map_err(|e| e.to_string())?
        .fetch()
        .map_err(|e| e.to_string())?;

    let versions: Vec<&str> = list.all().iter().map(|r| r.version()).collect();
    Ok(pick_upgrade(channel, current_version(), versions).map(str::to_owned))
}

#[cfg(feature = "updates")]
pub fn install(channel: Channel) -> UpdateStatus {
    let version = match latest_version(channel) {
        Ok(Some(version)) => version,
        Ok(None) => return UpdateStatus::UpToDate,
        Err(error) => return UpdateStatus::Failed { error },
    };

    let result = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(self_update::get_target())
        .current_version(current_version())
        .release_tag(format!("v{version}"))
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .build()
        .and_then(|updater| updater.update());

    match result {
        Ok(_) => UpdateStatus::Installed { version },
        Err(e) => UpdateStatus::Failed { error: e.to_string() },
    }
}
