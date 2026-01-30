//! Структуры данных для отслеживания прогресса индексации и восстановления.
//!
//! Поддерживает парсинг:
//! - Платформенных типов (Syntax Helper): 4 фазы
//! - Конфигурации (Configuration.xml): 5 фаз (включая индексацию BSL-модулей)
//! - Восстановления .hbk файлов: извлечение архивов

use serde::{Deserialize, Serialize};

/// Источник прогресса индексации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressSource {
    /// Типы платформы 1С (из Syntax Helper)
    Platform,
    /// Типы конфигурации (из Configuration.xml)
    Configuration,
}

/// Этапы парсинга (платформы и конфигурации)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexingPhase {
    // === ФАЗЫ ДЛЯ HBK RECOVERY ===
    /// Извлечение файлов из .hbk архивов (0-100%)
    HbkExtraction,

    // === ФАЗЫ ДЛЯ ПЛАТФОРМЫ (Syntax Helper) ===
    /// Сканирование файловой системы (0-10%)
    CollectingFiles,

    /// Парсинг HTML файлов Syntax Helper (10-70%)
    ParsingFiles,

    /// Связывание типов с категориями (70-90%)
    LinkingCategories,

    /// Построение индексов для быстрого поиска (90-100%)
    BuildingIndexes,

    // === ФАЗЫ ДЛЯ КОНФИГУРАЦИИ (Configuration.xml) ===
    /// Сканирование структуры конфигурации (0-5%)
    ConfigurationDiscovery,

    /// Парсинг XML файлов конфигурации (5-80%)
    ConfigurationParsing,

    /// Построение связей между типами (80-90%)
    ConfigurationLinking,

    /// Финализация загрузки конфигурации (90-95%)
    ConfigurationFinalizing,

    /// Индексация BSL-модулей конфигурации (95-100%)
    ConfigurationIndexingModules,
}

impl IndexingPhase {
    /// Возвращает источник прогресса (Platform или Configuration)
    pub fn source(&self) -> ProgressSource {
        match self {
            Self::HbkExtraction
            | Self::CollectingFiles
            | Self::ParsingFiles
            | Self::LinkingCategories
            | Self::BuildingIndexes => ProgressSource::Platform,

            Self::ConfigurationDiscovery
            | Self::ConfigurationParsing
            | Self::ConfigurationLinking
            | Self::ConfigurationFinalizing
            | Self::ConfigurationIndexingModules => ProgressSource::Configuration,
        }
    }

    /// Возвращает базовый процент начала фазы (0.0-100.0)
    pub fn base_percentage(&self) -> f32 {
        match self {
            // HBK Recovery
            Self::HbkExtraction => 0.0,

            // Платформа
            Self::CollectingFiles => 0.0,
            Self::ParsingFiles => 10.0,
            Self::LinkingCategories => 70.0,
            Self::BuildingIndexes => 90.0,

            // Конфигурация
            Self::ConfigurationDiscovery => 0.0,
            Self::ConfigurationParsing => 5.0,
            Self::ConfigurationLinking => 80.0,
            Self::ConfigurationFinalizing => 90.0,
            Self::ConfigurationIndexingModules => 95.0,
        }
    }

    /// Возвращает вклад фазы в общий процент (сумма всех весов = 100.0)
    pub fn weight(&self) -> f32 {
        match self {
            // HBK Recovery
            Self::HbkExtraction => 100.0,

            // Платформа
            Self::CollectingFiles => 10.0,
            Self::ParsingFiles => 60.0, // Основная фаза (самая долгая)
            Self::LinkingCategories => 20.0,
            Self::BuildingIndexes => 10.0,

            // Конфигурация
            Self::ConfigurationDiscovery => 5.0,
            Self::ConfigurationParsing => 75.0, // Основная фаза парсинга XML
            Self::ConfigurationLinking => 10.0,
            Self::ConfigurationFinalizing => 5.0,
            Self::ConfigurationIndexingModules => 5.0,
        }
    }

    /// Человекочитаемое название фазы (для русского интерфейса)
    pub fn display_name(&self) -> &'static str {
        match self {
            // HBK Recovery
            Self::HbkExtraction => "Извлечение файлов справки",

            // Платформа
            Self::CollectingFiles => "Сканирование файлов",
            Self::ParsingFiles => "Парсинг Syntax Helper",
            Self::LinkingCategories => "Связывание категорий",
            Self::BuildingIndexes => "Построение индексов",

            // Конфигурация
            Self::ConfigurationDiscovery => "Сканирование конфигурации",
            Self::ConfigurationParsing => "Парсинг Configuration.xml",
            Self::ConfigurationLinking => "Построение связей",
            Self::ConfigurationFinalizing => "Финализация",
            Self::ConfigurationIndexingModules => "Индексация BSL-модулей",
        }
    }
}

/// Обновление прогресса индексации или восстановления
///
/// Используется для отправки информации о ходе работы в VSCode UI.
/// Сериализуется в JSON с использованием тега для типа.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ProgressUpdateType {
    /// Прогресс парсинга файлов
    #[serde(rename = "parsing")]
    Parsing {
        /// Текущая фаза индексации
        phase: IndexingPhaseData,
        /// Текущий элемент (номер обработанного файла/типа)
        current: usize,
        /// Всего элементов в текущей фазе
        total: usize,
        /// Общий процент выполнения (0.0-100.0)
        percentage: f32,
        /// Человекочитаемое сообщение (например, имя текущего типа)
        message: Option<String>,
    },

    /// Прогресс извлечения файлов из .hbk архива
    #[serde(rename = "hbk_extraction")]
    HbkExtraction {
        /// Имя .hbk файла (например, "shcntx_ru")
        file_name: String,
        /// Количество уже извлечённых файлов
        extracted_files: usize,
        /// Общее количество файлов в архиве
        total_files: usize,
        /// Процент выполнения (0-100)
        percentage: u8,
    },
}

/// Сериализуемые данные о фазе индексации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingPhaseData {
    /// Текстовое представление фазы
    pub name: String,
}

/// Обновление прогресса парсинга (наследование для обратной совместимости)
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Текущая фаза индексации
    pub phase: IndexingPhase,

    /// Текущий элемент (номер обработанного файла/типа)
    pub current: usize,

    /// Всего элементов в текущей фазе
    pub total: usize,

    /// Общий процент выполнения (0.0-100.0)
    pub percentage: f32,

    /// Человекочитаемое сообщение (например, имя текущего типа)
    pub message: Option<String>,
}

impl ProgressUpdate {
    /// Вычисляет общий процент на основе фазы и прогресса внутри фазы
    ///
    /// # Формула
    /// percentage = base_percentage + (current / total) * weight
    ///
    /// # Примеры
    /// ```
    /// use bsl_runtime::data::loaders::progress::{IndexingPhase, ProgressUpdate};
    ///
    /// // Фаза ParsingFiles: обработано 1963 из 3927 файлов
    /// let percent = ProgressUpdate::compute_percentage(
    ///     IndexingPhase::ParsingFiles,
    ///     1963,
    ///     3927
    /// );
    /// assert_eq!(percent, 40.0); // 10% (base) + 50% * 60% (weight)
    /// ```
    pub fn compute_percentage(phase: IndexingPhase, current: usize, total: usize) -> f32 {
        let base = phase.base_percentage();
        let weight = phase.weight();

        // Защита от деления на 0
        if total == 0 {
            return base;
        }

        // Процент внутри фазы (0.0-1.0)
        let phase_progress = (current as f32 / total as f32).min(1.0);

        // Итоговый процент
        let percentage = base + phase_progress * weight;

        // ✅ НОВОЕ: Округление до целого числа (более стабильная прогрессия для UI)
        percentage.round()
    }

    /// Создаёт новое обновление прогресса с автоматическим вычислением процента
    pub fn new(
        phase: IndexingPhase,
        current: usize,
        total: usize,
        message: Option<String>,
    ) -> Self {
        let percentage = Self::compute_percentage(phase, current, total);

        Self {
            phase,
            current,
            total,
            percentage,
            message,
        }
    }

    /// Конвертирует в сериализуемый enum для отправки в UI
    pub fn to_update_type(&self) -> ProgressUpdateType {
        ProgressUpdateType::Parsing {
            phase: IndexingPhaseData {
                name: self.phase.display_name().to_string(),
            },
            current: self.current,
            total: self.total,
            percentage: self.percentage,
            message: self.message.clone(),
        }
    }
}

impl ProgressUpdateType {
    /// Создаёт обновление прогресса для извлечения .hbk архива
    pub fn hbk_extraction(
        file_name: impl Into<String>,
        extracted_files: usize,
        total_files: usize,
    ) -> Self {
        let percentage = if total_files == 0 {
            0
        } else {
            ((extracted_files as f32 / total_files as f32) * 100.0).round() as u8
        };

        Self::HbkExtraction {
            file_name: file_name.into(),
            extracted_files,
            total_files,
            percentage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_phase_base_percentage() {
        assert_eq!(IndexingPhase::CollectingFiles.base_percentage(), 0.0);
        assert_eq!(IndexingPhase::ParsingFiles.base_percentage(), 10.0);
        assert_eq!(IndexingPhase::LinkingCategories.base_percentage(), 70.0);
        assert_eq!(IndexingPhase::BuildingIndexes.base_percentage(), 90.0);
    }

    #[test]
    fn test_indexing_phase_weight() {
        // Платформа
        assert_eq!(IndexingPhase::CollectingFiles.weight(), 10.0);
        assert_eq!(IndexingPhase::ParsingFiles.weight(), 60.0);
        assert_eq!(IndexingPhase::LinkingCategories.weight(), 20.0);
        assert_eq!(IndexingPhase::BuildingIndexes.weight(), 10.0);

        // Сумма весов платформы = 100.0
        let total_weight_platform = IndexingPhase::CollectingFiles.weight()
            + IndexingPhase::ParsingFiles.weight()
            + IndexingPhase::LinkingCategories.weight()
            + IndexingPhase::BuildingIndexes.weight();
        assert_eq!(total_weight_platform, 100.0);

        // Конфигурация
        assert_eq!(IndexingPhase::ConfigurationDiscovery.weight(), 5.0);
        assert_eq!(IndexingPhase::ConfigurationParsing.weight(), 75.0);
        assert_eq!(IndexingPhase::ConfigurationLinking.weight(), 10.0);
        assert_eq!(IndexingPhase::ConfigurationFinalizing.weight(), 5.0);
        assert_eq!(IndexingPhase::ConfigurationIndexingModules.weight(), 5.0);

        // Сумма весов конфигурации = 100.0
        let total_weight_config = IndexingPhase::ConfigurationDiscovery.weight()
            + IndexingPhase::ConfigurationParsing.weight()
            + IndexingPhase::ConfigurationLinking.weight()
            + IndexingPhase::ConfigurationFinalizing.weight()
            + IndexingPhase::ConfigurationIndexingModules.weight();
        assert_eq!(total_weight_config, 100.0);
    }

    #[test]
    fn test_indexing_phase_display_name() {
        assert_eq!(
            IndexingPhase::CollectingFiles.display_name(),
            "Сканирование файлов"
        );
        assert_eq!(
            IndexingPhase::ParsingFiles.display_name(),
            "Парсинг Syntax Helper"
        );
        assert_eq!(
            IndexingPhase::LinkingCategories.display_name(),
            "Связывание категорий"
        );
        assert_eq!(
            IndexingPhase::BuildingIndexes.display_name(),
            "Построение индексов"
        );
    }

    #[test]
    fn test_compute_percentage_collecting_files() {
        // Начало фазы: 0/100
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 0, 100);
        assert_eq!(percent, 0.0);

        // Середина фазы: 50/100
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 50, 100);
        assert_eq!(percent, 5.0); // 0% (base) + 50% * 10% (weight)

        // Конец фазы: 100/100
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 100, 100);
        assert_eq!(percent, 10.0);
    }

    #[test]
    fn test_compute_percentage_parsing_files() {
        // Начало фазы: 0/3927
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 0, 3927);
        assert_eq!(percent, 10.0);

        // Середина фазы: 1963/3927 (50%)
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 1963, 3927);
        assert_eq!(percent, 40.0); // 10% (base) + 50% * 60% (weight) = 40.0, округлено до 40

        // Конец фазы: 3927/3927
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 3927, 3927);
        assert_eq!(percent, 70.0);
    }

    #[test]
    fn test_compute_percentage_linking_categories() {
        // Середина фазы: 50/100
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::LinkingCategories, 50, 100);
        assert_eq!(percent, 80.0); // 70% (base) + 50% * 20% (weight)
    }

    #[test]
    fn test_compute_percentage_building_indexes() {
        // Конец фазы: 3927/3927
        let percent =
            ProgressUpdate::compute_percentage(IndexingPhase::BuildingIndexes, 3927, 3927);
        assert_eq!(percent, 100.0);
    }

    #[test]
    fn test_compute_percentage_edge_cases() {
        // Деление на 0
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 10, 0);
        assert_eq!(percent, 10.0); // Возвращает base_percentage

        // current > total (защита от overflow)
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 5000, 3927);
        assert_eq!(percent, 70.0); // Максимум = base + weight
    }

    #[test]
    fn test_progress_update_new() {
        let update = ProgressUpdate::new(
            IndexingPhase::ParsingFiles,
            1963,
            3927,
            Some("Массив".to_string()),
        );

        assert_eq!(update.phase, IndexingPhase::ParsingFiles);
        assert_eq!(update.current, 1963);
        assert_eq!(update.total, 3927);
        assert_eq!(update.percentage, 40.0); // Округлено до целого
        assert_eq!(update.message, Some("Массив".to_string()));
    }
}
