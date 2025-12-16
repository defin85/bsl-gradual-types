//! Загрузка метаданных конфигураций
//!
//! Методы для загрузки метаданных из базовых конфигураций и расширений

use anyhow::Result;
use std::path::Path;
use tracing::info;

use crate::data::loaders::progress::ProgressUpdate;
use bsl_shared::domain::types::RawTypeData;

use super::coordinator::SystemCoordinator;
use super::types::LoadMetadataResult;

impl SystemCoordinator {
    /// Загрузить метаданные конфигурации через универсальный парсер
    ///
    /// Использует ConfigurationDiscovery для автоматического обнаружения всех объектов метаданных
    /// и загружает их в TypeRepository текущего AnalysisEngine.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use bsl_backend::system::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// let config_path = Path::new("examples/conf/conf_test");
    /// let loaded = coordinator.load_configuration_metadata(config_path).unwrap();
    /// println!("Загружено {} объектов метаданных", loaded);
    /// ```
    pub fn load_configuration_metadata(&self, config_path: &Path) -> Result<usize> {
        use crate::data::loaders::ConfigurationDiscovery;

        info!("Загрузка метаданных конфигурации из {:?}", config_path);

        // Создаём discovery и обнаруживаем все объекты
        // show_progress = false - нет терминального прогресс-бара для этого метода
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);

        // Без progress_callback в публичном методе (для обратной совместимости)
        let metadata_objects = discovery
            .discover_all_metadata(None::<fn(crate::data::loaders::progress::ProgressUpdate)>)
            .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))?;

        info!("Обнаружено {} объектов метаданных", metadata_objects.len());

        // Получаем текущий AnalysisEngine или создаём новый
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;

        // Получаем TypeRepository из AnalysisEngine
        let repository = engine.get_repository();

        // Конвертируем все объекты в RawTypeData
        let raw_types: Vec<RawTypeData> = metadata_objects
            .into_iter()
            .flat_map(|obj| obj.to_raw_type_data_with_forms(None))
            .collect();

        let count = raw_types.len();

        // Загружаем все типы в репозиторий за один вызов
        repository.load_types(raw_types)?;

        info!("Загружено {} типов из конфигурации", count);
        Ok(count)
    }

    /// Загружает метаданные из ВСЕХ конфигураций с прогрессом (4 фазы для каждой)
    ///
    /// Новая версия с поддержкой прогресса через callback.
    /// Автоматически обнаруживает все конфигурации в указанной папке:
    /// - Базовую конфигурацию (ObjectBelonging = "Own")
    /// - Расширения конфигурации (ObjectBelonging = "Adopted")
    ///
    /// Для каждой конфигурации отправляет прогресс через 4 фазы:
    /// - ConfigurationDiscovery (0-5%)
    /// - ConfigurationParsing (5-85%)
    /// - ConfigurationLinking (85-95%)
    /// - ConfigurationFinalizing (95-100%)
    ///
    /// # Аргументы
    /// * `config_path` - Путь к папке с конфигурациями (содержит Configuration.xml или подпапки)
    /// * `progress_callback` - Callback для отслеживания прогресса
    ///
    /// # Возвращает
    /// * `LoadMetadataResult` - Статистика загруженных типов
    pub fn load_all_configurations_with_progress<F>(
        &self,
        config_path: &Path,
        progress_callback: F,
    ) -> Result<LoadMetadataResult>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        use crate::data::loaders::config_metadata_parser::{
            ConfigurationDiscovery, ConfigurationType,
        };

        info!(
            "[WITH PROGRESS] Обнаружение конфигураций в: {}",
            config_path.display()
        );

        // show_progress = true - показываем терминальный прогресс-бар для Web Server
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

        for config_info in configurations {
            info!(
                "Загрузка конфигурации: {} ({}{})",
                config_info.name,
                if config_info.is_base() {
                    "Base"
                } else {
                    "Extension"
                },
                config_info
                    .prefix
                    .as_ref()
                    .map(|p| format!(", префикс: {}", p))
                    .unwrap_or_default()
            );

            // НОВОЕ: Используем новый метод с прогрессом через 4 фазы
            let metadata = discovery
                .discover_metadata_in_configuration_with_progress(
                    &config_info,
                    progress_callback.clone(),
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

            let prefix = config_info.prefix.as_deref();
            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .flat_map(|obj| obj.to_raw_type_data_with_forms(prefix))
                .collect();

            total_types += raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        Ok(LoadMetadataResult {
            base_config_count: base_count,
            extensions_count: ext_count,
            total_types,
        })
    }

    /// Загружает метаданные из ВСЕХ конфигураций (базовой + расширений) с применением префиксов
    ///
    /// Автоматически обнаруживает все конфигурации в указанной папке:
    /// - Базовую конфигурацию (ObjectBelonging = "Own")
    /// - Расширения конфигурации (ObjectBelonging = "Adopted")
    ///
    /// Для каждого расширения извлекает префикс из метаданных и применяет его к именам объектов,
    /// создавая корректные имена типов вида "Справочники.Префикс_Имя".
    ///
    /// # Аргументы
    /// * `config_path` - Путь к папке с конфигурациями (содержит Configuration.xml или подпапки)
    ///
    /// # Возвращает
    /// * `LoadMetadataResult` - Статистика загруженных типов
    ///
    /// # Примеры
    /// ```ignore
    /// use std::path::Path;
    /// use bsl_backend::system::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// coordinator.start().await.unwrap();
    ///
    /// let config_path = Path::new("examples/conf");
    /// let result = coordinator.load_all_configurations_metadata(config_path).unwrap();
    ///
    /// println!("Загружено {} типов", result.total_types);
    /// println!("Базовых конфигураций: {}", result.base_config_count);
    /// println!("Расширений: {}", result.extensions_count);
    /// ```
    pub fn load_all_configurations_metadata(
        &self,
        config_path: &Path,
    ) -> Result<LoadMetadataResult> {
        use crate::data::loaders::config_metadata_parser::{
            ConfigurationDiscovery, ConfigurationType,
        };

        info!("Обнаружение конфигураций в: {}", config_path.display());

        // show_progress = true - показываем терминальный прогресс-бар при парсинге конфигурации
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

        for config_info in configurations {
            info!(
                "Загрузка конфигурации: {} ({}{})",
                config_info.name,
                if config_info.is_base() {
                    "Base"
                } else {
                    "Extension"
                },
                config_info
                    .prefix
                    .as_ref()
                    .map(|p| format!(", префикс: {}", p))
                    .unwrap_or_default()
            );

            // Без progress_callback в публичном методе (для обратной совместимости)
            let metadata = discovery
                .discover_metadata_in_configuration(
                    &config_info,
                    None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

            let prefix = config_info.prefix.as_deref();
            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .flat_map(|obj| obj.to_raw_type_data_with_forms(prefix))
                .collect();

            total_types += raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        Ok(LoadMetadataResult {
            base_config_count: base_count,
            extensions_count: ext_count,
            total_types,
        })
    }
}
