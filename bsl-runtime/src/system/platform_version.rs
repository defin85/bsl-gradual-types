use std::fmt::Display;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PlatformVersion {
    pub(crate) major: u32,
    pub(crate) minor: u32,
    pub(crate) patch: u32,
}

impl Display for PlatformVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub(crate) fn format_platform_version(version: PlatformVersion) -> String {
    version.to_string()
}

pub(crate) fn parse_platform_version(raw: &str) -> Option<PlatformVersion> {
    let trimmed = raw.trim();
    let without_prefix = trimmed.strip_prefix("Version").unwrap_or(trimmed);
    let normalized = without_prefix.replace('_', ".");
    let mut parts = normalized.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some(PlatformVersion {
        major,
        minor,
        patch,
    })
}

pub(crate) fn normalize_platform_version(raw: &str) -> Option<String> {
    parse_platform_version(raw).map(format_platform_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_platform_version_accepts_normalized_and_prefixed_forms() {
        let direct = parse_platform_version("8.3.25").expect("direct");
        assert_eq!(direct.to_string(), "8.3.25");

        let prefixed = parse_platform_version("Version8_3_25").expect("prefixed");
        assert_eq!(prefixed.to_string(), "8.3.25");
    }

    #[test]
    fn parse_platform_version_rejects_invalid_forms() {
        assert!(parse_platform_version("Version8_3").is_none());
        assert!(parse_platform_version("invalid").is_none());
    }
}
