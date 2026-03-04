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
#[path = "platform_version/tests.rs"]
mod tests;
