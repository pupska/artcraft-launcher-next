//! ArtCraft Launcher updating itself.
//!
//! Delegated to [`craft_update::self_update`].

use std::ffi::OsString;

use artcraft_launcher_core::PackageKind;
use artcraft_launcher_core::platform::Platform;
use artcraft_launcher_core::protocol::JobPhase;
use artcraft_launcher_core::release::Release;
use artcraft_launcher_core::version::Version;

use crate::http::{Http, Stop};
use crate::paths::Paths;

pub use craft_update::self_update::{FINISH_FLAG, Install, PRETEND_VAR, is_self_updating};

/// The version this launcher counts as (see [`PRETEND_VAR`]).
pub fn current_version() -> Option<Version> {
    craft_update::self_update::current_version(&craft_update::suite::LAUNCHER, env!("CARGO_PKG_VERSION"))
}

/// Whether this build replaces itself when a newer launcher is out. Off in this fork: its
/// releases are unsigned, and `craft_update` installs only an update from
/// `storytold/craft-launcher` that is signed like the running copy (Windows) or by ArtCraft's
/// Apple team (macOS). Turned on, an ad-hoc signed fork build would accept upstream's signed DMG
/// and replace itself with upstream's launcher. Instead the UI offers the new release's download
/// for this platform ("Download" in the update banner).
pub const UPDATES_IN_PLACE: bool = false;

/// How this copy is installed, if it's one the launcher knows how to replace; always `None`
/// while [`UPDATES_IN_PLACE`] is off.
pub fn current() -> Option<Install> {
    if !UPDATES_IN_PLACE {
        return None;
    }
    craft_update::self_update::current(&craft_update::suite::LAUNCHER)
}

fn to_update_paths(paths: &Paths) -> craft_update::Paths {
    craft_update::Paths { data: paths.data.clone(), cache: paths.cache.clone() }
}

fn to_update_release(release: &Release) -> craft_update::Release {
    craft_update::Release {
        tag: release.tag.clone(),
        version: release.version.clone(),
        name: release.name.clone(),
        published_at: release.published_at,
        html_url: release.html_url.clone(),
        notes: release.notes.clone(),
        prerelease: release.prerelease,
        assets: release
            .assets
            .iter()
            .map(|a| craft_update::Asset { name: a.name.clone(), size: a.size, url: a.url.clone(), sha256: a.sha256.clone() })
            .collect(),
    }
}

fn to_update_platform(platform: Platform) -> craft_update::Platform {
    craft_update::Platform {
        os: match platform.os {
            artcraft_launcher_core::Os::Windows => craft_update::Os::Windows,
            artcraft_launcher_core::Os::MacOs => craft_update::Os::MacOs,
            artcraft_launcher_core::Os::Linux => craft_update::Os::Linux,
            artcraft_launcher_core::Os::FreeBsd => craft_update::Os::FreeBsd,
        },
        arch: match platform.arch {
            artcraft_launcher_core::Arch::X64 => craft_update::Arch::X64,
            artcraft_launcher_core::Arch::Arm64 => craft_update::Arch::Arm64,
            artcraft_launcher_core::Arch::X86 => craft_update::Arch::X86,
            artcraft_launcher_core::Arch::Riscv64 => craft_update::Arch::Riscv64,
        },
    }
}

/// Whether `release` is the version this launcher just updated itself to.
pub fn just_updated_to(paths: &Paths, release: &Release) -> bool {
    let update_paths = to_update_paths(paths);
    let update_release = to_update_release(release);
    craft_update::self_update::just_updated_to(&update_paths, &update_release)
}

/// The error of the last update, if it failed, once.
pub fn take_failure(paths: &Paths) -> Option<String> {
    let update_paths = to_update_paths(paths);
    craft_update::self_update::take_failure(&update_paths)
}

/// Downloads `release` for `platform`, puts it in place and starts the helper that relaunches
/// it once this process has exited. On success the caller must quit promptly. `last` is called
/// once the download is ready and returns when the launcher may replace itself and restart.
#[allow(clippy::too_many_arguments)]
pub fn update(
    install: &Install,
    release: &Release,
    platform: Platform,
    _http: &Http,
    paths: &Paths,
    stop: &Stop,
    phase: &mut dyn FnMut(JobPhase),
    last: &dyn Fn() -> Result<(), String>,
) -> Result<(), String> {
    let update_release = to_update_release(release);
    let update_platform = to_update_platform(platform);
    let update_paths = to_update_paths(paths);
    let update_http = craft_update::Http::new("ArtCraft-Launcher/SelfUpdate")?;
    let mut update_phase = |p: craft_update::JobPhase| {
        phase(match p {
            craft_update::JobPhase::Downloading { done, total } => JobPhase::Downloading { done, total },
            craft_update::JobPhase::Reconnecting { done, total } => JobPhase::Reconnecting { done, total },
            craft_update::JobPhase::Waiting => JobPhase::Waiting,
            craft_update::JobPhase::Verifying => JobPhase::Verifying,
            craft_update::JobPhase::Installing => JobPhase::Installing,
            craft_update::JobPhase::Removing => JobPhase::Removing,
        });
    };
    craft_update::self_update::update(
        &craft_update::suite::LAUNCHER,
        install,
        &update_release,
        update_platform,
        &update_http,
        &update_paths,
        stop,
        &mut update_phase,
        last,
        Some(env!("CARGO_PKG_VERSION")),
    )
}

/// The package format updating `install` downloads: the one it was installed from. Mirrors the
/// private `Install::kind` in `craft_update::self_update`, which picks the update's asset by it.
pub fn kind(install: &Install) -> PackageKind {
    match install {
        Install::MacApp(_) => PackageKind::Dmg,
        Install::Msi(_) => PackageKind::Msi,
        Install::Portable(_) => PackageKind::PortableZip,
        Install::AppImage(_) => PackageKind::AppImage,
        Install::Package(kind, _) => match kind {
            craft_update::PackageKind::Msi => PackageKind::Msi,
            craft_update::PackageKind::PortableZip => PackageKind::PortableZip,
            craft_update::PackageKind::Dmg => PackageKind::Dmg,
            craft_update::PackageKind::AppImage => PackageKind::AppImage,
            craft_update::PackageKind::Deb => PackageKind::Deb,
            craft_update::PackageKind::Rpm => PackageKind::Rpm,
            craft_update::PackageKind::ArchPkg => PackageKind::ArchPkg,
            craft_update::PackageKind::Flatpak => PackageKind::Flatpak,
            craft_update::PackageKind::TarGz => PackageKind::TarGz,
        },
        Install::Flatpak => PackageKind::Flatpak,
        Install::Standalone(_) if cfg!(windows) => PackageKind::PortableZip,
        Install::Standalone(_) => PackageKind::TarGz,
    }
}

/// Finishes an update when this program was started as the update helper.
pub fn finish(args: &[OsString]) -> Option<Result<(), String>> {
    craft_update::self_update::finish(&craft_update::suite::LAUNCHER, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_development_build_does_not_update_itself() {
        assert_eq!(current(), None);
    }

    /// A fork build never replaces itself, even where `craft_update` would: with
    /// `CRAFT_UPDATE_DEV` set it treats this test binary as an installed copy (a standalone
    /// executable on Linux). The variable is set on a child run of the test below, as setting it
    /// here would need `unsafe`.
    #[test]
    fn a_fork_build_never_updates_itself_in_place() {
        let exe = std::env::current_exe().unwrap();
        let out = std::process::Command::new(exe)
            .env(craft_update::self_update::DEV_VAR, "1")
            .args(["--exact", "self_update::tests::opted_in_copy_is_not_replaced", "--include-ignored", "--nocapture"])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success() && log.contains("1 passed"), "{log}{}", String::from_utf8_lossy(&out.stderr));
    }

    /// Run by [`a_fork_build_never_updates_itself_in_place`] with `CRAFT_UPDATE_DEV` set.
    #[test]
    #[ignore = "run by a_fork_build_never_updates_itself_in_place, with CRAFT_UPDATE_DEV set"]
    fn opted_in_copy_is_not_replaced() {
        // Without the variable (a plain `--include-ignored` run) there is nothing to check.
        if std::env::var_os(craft_update::self_update::DEV_VAR).is_some() {
            assert_eq!(current(), None);
        }
    }

    #[test]
    fn an_update_downloads_the_format_the_copy_was_installed_from() {
        use std::path::PathBuf;
        assert_eq!(kind(&Install::Msi(PathBuf::new())), PackageKind::Msi);
        assert_eq!(kind(&Install::Portable(PathBuf::new())), PackageKind::PortableZip);
        assert_eq!(kind(&Install::MacApp(PathBuf::new())), PackageKind::Dmg);
        assert_eq!(kind(&Install::AppImage(PathBuf::new())), PackageKind::AppImage);
        assert_eq!(kind(&Install::Package(craft_update::PackageKind::Rpm, PathBuf::new())), PackageKind::Rpm);
        assert_eq!(kind(&Install::Flatpak), PackageKind::Flatpak);
        let standalone = if cfg!(windows) { PackageKind::PortableZip } else { PackageKind::TarGz };
        assert_eq!(kind(&Install::Standalone(PathBuf::new())), standalone);
    }
}
