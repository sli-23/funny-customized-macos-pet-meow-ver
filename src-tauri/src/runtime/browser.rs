use std::sync::OnceLock;

pub fn get_browser_url(app_name: &str) -> Option<String> {
    use crate::platform::{self, Platform};
    let plat = platform::native();
    plat.get_browser_url(app_name)
}

pub fn extract_site(title: &str) -> String {
    let lower = title.to_lowercase();
    let known_sites = [
        "github", "stackoverflow", "youtube", "twitter", "reddit",
        "linkedin", "google", "slack", "notion", "figma",
        "gitlab", "bitbucket", "jira", "confluence",
    ];
    for site in &known_sites {
        if lower.contains(site) {
            return site.to_string();
        }
    }
    title.split(" - ").last()
        .or_else(|| title.split(" — ").last())
        .unwrap_or(title)
        .trim()
        .to_lowercase()
}

pub fn extract_site_from_url(url: &str) -> String {
    let lower = url.to_lowercase();

    // Check dynamically loaded extra sites (from module drop-in)
    for site in load_extra_sites() {
        if lower.contains(site) {
            return site.to_string();
        }
    }

    let known_sites = [
        "github.com", "youtube.com", "twitter.com",
        "reddit.com", "linkedin.com", "bilibili.com",
        "google.com", "figma.com",
    ];
    for site in &known_sites {
        if lower.contains(site) {
            return site.split('.').next().unwrap_or(site).to_string();
        }
    }

    if let Some(start) = lower.find("://") {
        let domain_part = &lower[start + 3..];
        if let Some(end) = domain_part.find('/') {
            return domain_part[..end].to_string();
        }
        return domain_part.to_string();
    }
    lower
}

static EXTRA_SITES: OnceLock<Vec<String>> = OnceLock::new();

fn load_extra_sites() -> &'static Vec<String> {
    EXTRA_SITES.get_or_init(|| {
        let modules_dir = super::module_loader::default_modules_dir();
        let sites_path = modules_dir.join("amazon-internal").join("sites.json");
        if let Ok(data) = std::fs::read_to_string(&sites_path) {
            serde_json::from_str::<Vec<String>>(&data).unwrap_or_default()
        } else {
            Vec::new()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_site_github() {
        assert_eq!(extract_site("Pull Request - GitHub"), "github");
    }

    #[test]
    fn test_extract_site_unknown() {
        assert_eq!(extract_site("Random Page - MyApp"), "myapp");
    }

    #[test]
    fn test_extract_site_from_url_github() {
        assert_eq!(extract_site_from_url("https://github.com/user/repo"), "github");
    }

    #[test]
    fn test_extract_site_from_url_unknown_domain() {
        assert_eq!(
            extract_site_from_url("https://example.org/page"),
            "example.org"
        );
    }

    #[test]
    fn test_extract_site_from_url_bilibili() {
        assert_eq!(
            extract_site_from_url("https://www.bilibili.com/video/123"),
            "bilibili"
        );
    }
}
