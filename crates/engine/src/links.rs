//! Patronus community and project links (Help menu, About, start screen, CLI, MCP).

use serde_json::{Value, json};

/// The app's slug, which is also the name of its GitHub repository.
pub const APP: &str = "designcraft";
/// Where the community talks: the repository's GitHub Discussions. (The constant and its `discord`
/// key keep the name they have always had, so the commands and `app.links` stay compatible.)
pub const DISCORD: &str = "https://github.com/darcon2k12345/designcraft/discussions";
/// Patronus on GitHub. `{WEBSITE}/{slug}` is an app's repository.
pub const WEBSITE: &str = "https://github.com/darcon2k12345";
/// Where Patronus Design's installers are published.
pub const APP_PAGE: &str = "https://github.com/darcon2k12345/designcraft/releases";
/// Patronus Design's source repository.
pub const GITHUB: &str = "https://github.com/darcon2k12345/designcraft";
/// Where to report bugs and request features.
pub const ISSUES: &str = "https://github.com/darcon2k12345/designcraft/issues";

/// All links as JSON (the `app.links` command).
pub fn all() -> Value {
    json!({
        "discord": DISCORD,
        "website": WEBSITE,
        "appPage": APP_PAGE,
        "github": GITHUB,
        "issues": ISSUES,
    })
}

/// A link by key (`discord`, `website`, `appPage`, `github`, `issues`).
pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        "discord" => DISCORD,
        "website" => WEBSITE,
        "appPage" => APP_PAGE,
        "github" => GITHUB,
        "issues" => ISSUES,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_follow_the_app_slug() {
        assert!(APP_PAGE.starts_with(GITHUB) && APP_PAGE.ends_with("/releases"));
        assert!(DISCORD.starts_with(GITHUB) && WEBSITE.starts_with("https://github.com/"));
        assert!(GITHUB.ends_with(&format!("/{APP}")));
        assert!(ISSUES.starts_with(GITHUB));
        for k in ["discord", "website", "appPage", "github", "issues"] {
            assert!(get(k).is_some_and(|u| u.starts_with("https://")), "{k}");
            assert_eq!(all()[k], get(k).unwrap());
        }
    }
}
