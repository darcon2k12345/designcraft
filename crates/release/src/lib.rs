//! How release tooling knows Patronus Design: its id, display name and the repository its releases
//! are downloaded from.
//!
//! The app's own updater and the launcher both use [`APP`], so they can never disagree about the
//! asset names to look for or where to look. The app was called DesignCraft before it became
//! Patronus Design; the old id and name are listed as former ones so that existing installs are still
//! recognised and old release assets (`designcraft-<version>-…`) are still found.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

use craft_update::app::App;

/// The repository releases come from unless a build says otherwise (see [`RELEASE_REPO`]).
pub const DEFAULT_RELEASE_REPO: &str = "darcon2k12345/designcraft";

/// The GitHub `owner/repo` whose releases the updater and launcher trust.
///
/// Fixed when the program is built, from the `PATRONUS_RELEASE_REPO` environment variable, so
/// that nothing at run time can redirect updates to another repository.
pub const RELEASE_REPO: &str = match option_env!("PATRONUS_RELEASE_REPO") {
    Some(repo) => repo,
    None => DEFAULT_RELEASE_REPO,
};

/// Patronus Design as craft-update knows it (no icon or accent colour; the GUI adds those).
pub const APP: App = App {
    former_ids: &["designcraft"],
    former_names: &["DesignCraft"],
    developer: "Patronus",
    bundle_id_override: Some("com.patronus.patronusdesign"),
    ..App::new("patronusdesign", "Patronus Design", "Page layout and publishing", RELEASE_REPO)
};

/// PdfCraft, installed from its own fork. (Registry entry from craft-update, repository swapped.)
pub const PDFCRAFT: App = App { repo: "darcon2k12345/pdfcraft", developer: "Patronus", ..craft_update::suite::PDFCRAFT };

/// PhotoCraft, installed from its own fork.
pub const PHOTOCRAFT: App = App { repo: "darcon2k12345/photocraft", developer: "Patronus", ..craft_update::suite::PHOTOCRAFT };

/// Every app the launcher manages, in display order.
pub static APPS: [App; 3] = [APP, PDFCRAFT, PHOTOCRAFT];

/// Finds a managed app by id, former id (`designcraft`, `printcraft`) or name, ignoring case.
pub fn find(key: &str) -> Option<&'static App> {
    APPS.iter()
        .find(|a| a.id.eq_ignore_ascii_case(key) || a.name.eq_ignore_ascii_case(key) || a.former_ids.iter().any(|f| f.eq_ignore_ascii_case(key)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_former_names() {
        assert_eq!(APP.id, "patronusdesign");
        assert_eq!(APP.name, "Patronus Design");
        assert_eq!(APP.bundle_id(), "com.patronus.patronusdesign");
        assert_eq!(APP.stems().collect::<Vec<_>>(), ["patronusdesign", "designcraft"]);
        assert_eq!(APP.names().collect::<Vec<_>>(), ["Patronus Design", "DesignCraft"]);
    }

    #[test]
    fn repo_looks_like_owner_slash_name() {
        let (owner, name) = RELEASE_REPO.split_once('/').expect("owner/repo");
        assert!(!owner.is_empty() && !name.is_empty() && !name.contains('/'));
    }

    #[test]
    fn apps_are_found_by_current_and_former_names() {
        assert_eq!(find("patronusdesign").map(|a| a.id), Some("patronusdesign"));
        assert_eq!(find("DesignCraft").map(|a| a.id), Some("patronusdesign"));
        assert_eq!(find("Patronus Design").map(|a| a.id), Some("patronusdesign"));
        assert_eq!(find("printcraft").map(|a| a.id), Some("pdfcraft"));
        assert_eq!(find("PhotoCraft").map(|a| a.id), Some("photocraft"));
        assert!(find("wordcraft").is_none() && find("").is_none());
    }

    #[test]
    fn app_ids_are_unique_and_forks_are_named_after_their_app() {
        let mut ids: Vec<_> = APPS.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), APPS.len(), "ids must be unique");
        for a in [&PDFCRAFT, &PHOTOCRAFT] {
            assert!(a.repo.starts_with("darcon2k12345/") && a.repo.ends_with(a.id), "{} -> {}", a.id, a.repo);
        }
        assert_eq!(PDFCRAFT.repo, "darcon2k12345/pdfcraft");
        assert_eq!(PHOTOCRAFT.repo, "darcon2k12345/photocraft");
    }
}
