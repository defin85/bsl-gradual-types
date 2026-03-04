use super::*;

impl RuntimeKey {
    pub fn env(self) -> &'static str {
        self.spec().env
    }

    pub(super) fn index(self) -> usize {
        RuntimeKey::ALL
            .iter()
            .position(|k| *k == self)
            .expect("RuntimeKey present in ALL")
    }

    pub(super) fn mutability(self) -> KeyMutability {
        match self {
            RuntimeKey::CacheDir
            | RuntimeKey::AgentHttpAddr
            | RuntimeKey::AgentHttpStaticDir
            | RuntimeKey::WebHost
            | RuntimeKey::WebPort
            | RuntimeKey::StaticPath
            | RuntimeKey::ProjectPath
            | RuntimeKey::PlatformVersion
            | RuntimeKey::SyntaxHelperPath => KeyMutability::StartupOnly,
            _ => KeyMutability::Runtime,
        }
    }
}
