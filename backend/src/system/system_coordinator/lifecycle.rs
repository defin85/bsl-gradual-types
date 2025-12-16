//! Lifecycle management для SystemCoordinator
//!
//! Инициализация системы, загрузка типов платформы

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{RawDataSource, RawTypeData};
use bsl_shared::engine::AnalysisEngine;

use crate::data::adapters::convert_syntax_helper_to_raw;
use crate::data::loaders::{hbk_recovery, progress::ProgressUpdate, SyntaxHelperLoader};
use crate::system::parser_coordinator::ParserCoordinator;

use super::coordinator::SystemCoordinator;
use super::types::StartupError;

impl SystemCoordinator {
    /// Инициализация системы с реальным парсингом синтаксис-помощника
    pub async fn start(&self) -> Result<(), StartupError> {
        self.start_with_paths(None, None, None).await
    }

    /// Инициализация системы с настраиваемыми путями (async версия)
    ///
    /// ВНИМАНИЕ: Эта функция выполняется в основном tokio event loop.
    /// Для CPU-intensive парсинга используйте start_with_paths_blocking() через spawn_blocking()
    pub async fn start_with_paths(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        info!("Starting platform types parser in blocking thread...");

        // Делегируем синхронной версии через spawn_blocking для предотвращения блокировки event loop
        let coordinator = self.clone_for_blocking();
        let syntax_path_owned = syntax_helper_path.map(|p| p.to_path_buf());
        let config_path_owned = config_path.map(|p| p.to_path_buf());

        let parser_handle = tokio::task::spawn_blocking(move || {
            info!("[BLOCKING THREAD] Parser started");
            let result = coordinator.start_with_paths_blocking(
                syntax_path_owned.as_deref(),
                config_path_owned.as_deref(),
                progress_tx,
            );
            info!("[BLOCKING THREAD] Parser finished");
            result
        });

        info!("Parser spawned in blocking thread, event loop remains free");

        match parser_handle.await {
            Ok(result) => result,
            Err(e) => Err(StartupError::CacheError(format!(
                "Blocking task panicked: {}",
                e
            ))),
        }
    }

    /// Синхронная версия инициализации системы (для spawn_blocking)
    ///
    /// Выполняет CPU-intensive парсинг типов платформы в отдельном блокирующем потоке,
    /// не блокируя основной tokio event loop.
    pub fn start_with_paths_blocking(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("[BLOCKING THREAD] SystemCoordinator: инициализация System Layer...");

        // КРИТИЧЕСКИ ВАЖНО: Очищаем кеши при повторной инициализации
        // Это гарантирует, что TypeSystemService получит НОВЫЙ AnalysisEngine с НОВЫМ TypeRepository
        // Соблюдаем lock order convention: analysis_engine_cache -> type_service_cache
        {
            let mut engine_cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            let mut service_cache = self.type_service_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Type service cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });

            if engine_cache.is_some() || service_cache.is_some() {
                info!("[BLOCKING THREAD] Очищаем кеши AnalysisEngine и TypeSystemService для повторной инициализации");
                *engine_cache = None;
                *service_cache = None;
            }
        }

        // === PHASE 3: Infrastructure инициализация в SystemCoordinator ===

        // 1. Создаем Infrastructure компоненты (Data Layer)
        info!("SystemCoordinator: инициализация Data Layer loaders...");
        let mut syntax_parser = SyntaxHelperLoader::new();

        // 2. Загружаем синтаксис-помощник если путь указан
        if let Some(syntax_path) = syntax_helper_path {
            self.load_syntax_helper(&mut syntax_parser, syntax_path, &progress_tx)?;
        }

        // 3. Создаем Domain Layer компоненты
        info!("SystemCoordinator: инициализация Domain Layer...");
        let repository = Arc::new(InMemoryTypeRepository::new());

        // 4. Загружаем данные в репозиторий (через Adapters)
        let database = syntax_parser.export_database();
        if !database.nodes.is_empty() {
            self.populate_repository_from_syntax_helper(&repository, database)?;
        } else {
            // Загружаем базовые типы как fallback
            Self::load_fallback_types(&repository)?;
        }

        // 5. Создаем Domain resolver
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        // MILESTONE 3.17: Обновляем ParserCoordinator с TypeResolver для резолюции active_facet
        {
            let new_parser = Arc::new(ParserCoordinator::new_with_resolver(
                repository.clone(),
                resolver.clone(),
            ));
            let mut parser_guard = self.parser.write().unwrap_or_else(|poisoned| {
                warn!("Parser RwLock poisoned (write), recovering data.");
                poisoned.into_inner()
            });
            *parser_guard = new_parser;
            info!("ParserCoordinator обновлён с TypeResolver для Milestone 3.17");
        }

        // 6. Создаем упрощенный AnalysisEngine (без Infrastructure зависимостей)
        let analysis_engine = AnalysisEngine::new(resolver, repository);

        // Кешируем AnalysisEngine
        {
            let mut cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            *cache = Some(Arc::new(analysis_engine));
        }

        // 7. Загружаем метаданные конфигурации если путь указан
        if let Some(config_path) = config_path {
            info!(
                "Загружаем метаданные конфигурации: {}",
                config_path.display()
            );

            // НОВОЕ: Используем версию с прогрессом если передан progress_tx
            let result = if let Some(ref tx) = progress_tx {
                let tx_clone = tx.clone();
                self.load_all_configurations_with_progress(config_path, move |update| {
                    let _ = tx_clone.send(update); // Отправляем прогресс в channel
                })
            } else {
                // Обратная совместимость: используем версию без прогресса
                self.load_all_configurations_metadata(config_path)
            };

            match result {
                Ok(result_data) => {
                    info!(
                        "Загружено {} типов из {} базовых конфигураций и {} расширений",
                        result_data.total_types,
                        result_data.base_config_count,
                        result_data.extensions_count
                    );
                }
                Err(e) => {
                    warn!("Ошибка загрузки метаданных конфигурации: {}", e);
                    info!("Продолжаем работу с типами платформы...");
                }
            }
        }

        info!("[BLOCKING THREAD] SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        info!("[BLOCKING THREAD] SystemCoordinator: система готова!");
        Ok(())
    }

    /// Загрузка синтаксис-помощника с HBK recovery
    fn load_syntax_helper(
        &self,
        syntax_parser: &mut SyntaxHelperLoader,
        syntax_path: &Path,
        progress_tx: &Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        // HBK Recovery: Восстанавливаем .hbk файлы перед парсингом
        info!("Проверяем наличие .hbk файлов для восстановления...");

        // PHASE 2: Интеграция Progress Callback для HBK Recovery
        let hbk_result = if let Some(ref tx) = progress_tx {
            let tx_clone = tx.clone();
            hbk_recovery::auto_recover_directory_with_progress(
                syntax_path,
                Some(
                    move |update: crate::data::loaders::progress::ProgressUpdateType| {
                        if let crate::data::loaders::progress::ProgressUpdateType::HbkExtraction {
                            file_name,
                            extracted_files,
                            total_files,
                            percentage,
                        } = update
                        {
                            let _ = tx_clone.send(crate::data::loaders::progress::ProgressUpdate {
                                phase: crate::data::loaders::progress::IndexingPhase::HbkExtraction,
                                current: extracted_files,
                                total: total_files,
                                percentage: percentage as f32,
                                message: Some(format!("Извлекаем файлы из {}", file_name)),
                            });
                        }
                    },
                ),
            )
        } else {
            hbk_recovery::auto_recover_directory(syntax_path)
        };

        match hbk_result {
            Ok(results) if !results.is_empty() => {
                info!("Восстановлено {} .hbk файлов", results.len());
                for result in &results {
                    info!(
                        "   Файл: {:?} -> {:?}",
                        result.repaired_zip_path.file_name().unwrap_or_default(),
                        result
                            .extracted_dir
                            .as_ref()
                            .map(|d| d.file_name().unwrap_or_default())
                    );
                }
            }
            Ok(_) => {
                info!(".hbk файлы не найдены (возможно уже распакованы)");
            }
            Err(e) => {
                warn!(
                    "Ошибка восстановления .hbk файлов: {}. Продолжаем с существующими файлами...",
                    e
                );
            }
        }

        info!("Загружаем синтаксис-помощник: {}", syntax_path.display());

        // MILESTONE 2.20.2.3: Парсим с прогрессом если передан callback
        if let Some(ref tx) = progress_tx {
            let tx_clone = tx.clone();
            match syntax_parser.parse_with_progress(syntax_path, move |update: ProgressUpdate| {
                let _ = tx_clone.send(update); // Отправляем в channel
            }) {
                Ok(()) => {
                    info!("Парсинг синтаксис-помощника завершен успешно");
                }
                Err(e) => {
                    warn!("Ошибка парсинга синтаксис-помощника: {}", e);
                    info!("Будем использовать базовые типы платформы 1С...");
                }
            }
        } else {
            // Обратная совместимость: парсим без прогресса
            match syntax_parser.parse_syntax_helper(syntax_path) {
                Ok(()) => {
                    info!("Парсинг синтаксис-помощника завершен успешно");
                }
                Err(e) => {
                    warn!("Ошибка парсинга синтаксис-помощника: {}", e);
                    info!("Будем использовать базовые типы платформы 1С...");
                }
            }
        }

        Ok(())
    }

    /// Заполнение репозитория из синтаксис-помощника
    fn populate_repository_from_syntax_helper(
        &self,
        repository: &Arc<InMemoryTypeRepository>,
        database: crate::data::loaders::syntax_helper::SyntaxHelperDatabase,
    ) -> Result<(), StartupError> {
        let platform_raw_data = convert_syntax_helper_to_raw(&database);

        // MILESTONE 2.20.5: Заполняем SignatureIndex из загруженных типов
        let platform_types_clone = platform_raw_data.clone(); // Клонируем для SignatureIndex

        repository
            .load_types(platform_raw_data)
            .map_err(StartupError::PlatformTypesError)?;

        // Заполняем SignatureIndex через Registry паттерн
        // Единственный источник данных - syntax_helper (документация 1С)
        use crate::data::loaders::{apply_generic_info_to_repository, SyntaxHelperSource};
        use bsl_shared::domain::SignatureSourceRegistry;

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone))
            .build();
        repository.set_signature_index(index);

        // Milestone 3.x: Применяем GenericInfo для типов-коллекций (inference rules)
        let generic_count = apply_generic_info_to_repository(repository.as_ref());

        let stats = repository.get_stats();
        info!(
            "Загружено {} типов из синтаксис-помощника",
            stats.total_types
        );
        info!("SignatureIndex заполнен платформенными методами");
        info!("GenericInfo применён к {} типам-коллекциям", generic_count);

        Ok(())
    }

    /// Загрузка базовых типов как fallback
    ///
    /// Используется когда syntax_helper не доступен.
    /// Загружает только примитивные типы и типы-коллекции без методов.
    /// Методы будут недоступны, но GenericInfo для inference будет работать.
    pub(crate) fn load_fallback_types(
        repository: &Arc<InMemoryTypeRepository>,
    ) -> Result<(), StartupError> {
        info!("Загружаем базовые типы платформы 1С (fallback mode)...");

        // Примитивные типы
        let platform_types = vec![
            RawTypeData {
                name: "Строка".to_string(),
                english_name: "String".to_string(),
                description: "Строковый тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Число".to_string(),
                english_name: "Number".to_string(),
                description: "Числовой тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Булево".to_string(),
                english_name: "Boolean".to_string(),
                description: "Логический тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Дата".to_string(),
                english_name: "Date".to_string(),
                description: "Тип данных для работы с датой и временем".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            // Типы-коллекции (без методов, только для GenericInfo)
            RawTypeData {
                name: "Массив".to_string(),
                english_name: "Array".to_string(),
                description: "Динамический массив".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Соответствие".to_string(),
                english_name: "Map".to_string(),
                description: "Ассоциативный массив (ключ-значение)".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "СписокЗначений".to_string(),
                english_name: "ValueList".to_string(),
                description: "Список значений с представлениями".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ТабличнаяЧасть".to_string(),
                english_name: "TabularSection".to_string(),
                description: "Табличная часть объекта".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ];

        let platform_types_clone = platform_types.clone();
        let type_count = platform_types.len();

        repository
            .load_types(platform_types)
            .map_err(StartupError::PlatformTypesError)?;

        // Заполняем SignatureIndex (будет пустой в fallback mode)
        use crate::data::loaders::{apply_generic_info_to_repository, SyntaxHelperSource};
        use bsl_shared::domain::SignatureSourceRegistry;

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone))
            .build();
        repository.set_signature_index(index);

        // Применяем GenericInfo для типов-коллекций
        let generic_count = apply_generic_info_to_repository(repository.as_ref());

        info!(
            "Базовые типы загружены: {} типов (fallback mode)",
            type_count
        );
        info!("GenericInfo применён к {} типам-коллекциям", generic_count);
        warn!("Методы недоступны в fallback mode. Укажите путь к syntax_helper для полной функциональности.");
        Ok(())
    }
}
