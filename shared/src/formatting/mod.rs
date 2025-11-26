//! MILESTONE 3.6 Phase 1: Форматирование hover с настраиваемыми уровнями детализации

/// Уровень детализации hover информации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    /// Только тип переменной (минимум)
    Compact,

    /// Тип + методы (до max_methods) - стандартный режим
    Full,

    /// Тип + методы + свойства + фасеты + документация (максимум)
    Detailed,
}

impl DetailLevel {
    /// Конвертация из строки (из VSCode settings)
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "compact" => Self::Compact,
            "detailed" => Self::Detailed,
            _ => Self::Full, // default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_level_from_str() {
        assert_eq!(DetailLevel::parse("compact"), DetailLevel::Compact);
        assert_eq!(DetailLevel::parse("full"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("detailed"), DetailLevel::Detailed);
        assert_eq!(DetailLevel::parse("FULL"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("unknown"), DetailLevel::Full); // default
    }
}
