//! Требования к контексту выполнения (для проверки вызовов методов).
//!
//! Этот тип используется в разных слоях (парсеры, индексы сигнатур, runtime),
//! поэтому живет в `bsl-types` как базовый доменный enum.

/// Требования к контексту выполнения (для проверки вызовов методов).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ContextRequirements {
    /// Только на сервере.
    ServerOnly,
    /// Только на клиенте.
    ClientOnly,
    /// Везде (универсальный код).
    #[default]
    Universal,
    /// Везде, но предпочтительно на сервере (для оптимизации).
    ServerPreferred,
}

impl ContextRequirements {
    /// Проверяет, является ли контекст универсальным.
    pub fn is_universal(&self) -> bool {
        matches!(self, Self::Universal | Self::ServerPreferred)
    }

    /// Проверяет, разрешено ли выполнение на сервере.
    pub fn allows_server(&self) -> bool {
        matches!(
            self,
            Self::ServerOnly | Self::Universal | Self::ServerPreferred
        )
    }

    /// Проверяет, разрешено ли выполнение на клиенте.
    pub fn allows_client(&self) -> bool {
        matches!(
            self,
            Self::ClientOnly | Self::Universal | Self::ServerPreferred
        )
    }
}
