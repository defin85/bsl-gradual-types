//! API поиска и валидации метаданных (Milestone 3.16).
//!
//! Предоставляет методы для:
//! - Проверки существования объектов метаданных
//! - Поиска похожих имён (fuzzy matching)
//! - Проверки загрузки конфигурации

use super::TypeMetadataLookup;
use crate::domain::types::MetadataKind;
use crate::domain::types::RawDataSource;
use crate::utils::string_utils::levenshtein_distance;

impl TypeMetadataLookup {
    /// Возвращает имена объектов метаданных указанного вида из repository-backed contract.
    pub fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String> {
        self.repository.get_metadata_objects_by_kind(kind)
    }

    /// Проверяет существование объекта метаданных указанного вида
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя объекта без префикса (например, "Контрагенты")
    ///
    /// # Возвращает
    ///
    /// `true` если объект найден в репозитории
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::repository::TypeRepository;
    /// # use bsl_shared::domain::types::MetadataKind;
    /// # use std::sync::Arc;
    /// # let repository: Arc<dyn TypeRepository> = todo!();
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// // Проверяем существующий справочник
    /// assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Контрагенты"));
    ///
    /// // Проверяем несуществующий справочник
    /// assert!(!lookup.exists_metadata_object(MetadataKind::Catalog, "НесуществующийСправочник"));
    /// ```
    pub fn exists_metadata_object(&self, kind: MetadataKind, name: &str) -> bool {
        let full_type_name = format!("{}.{}", kind.to_prefix(), name);
        self.repository.find_type(&full_type_name).is_some()
    }

    /// Возвращает похожие имена объектов метаданных (fuzzy matching)
    ///
    /// Использует алгоритм Левенштейна для поиска похожих имён.
    /// Полезно для диагностических сообщений с предложениями исправлений.
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя для поиска похожих
    /// * `max_suggestions` - максимальное количество предложений
    ///
    /// # Алгоритм
    ///
    /// 1. Получает все объекты указанного вида
    /// 2. Вычисляет расстояние Левенштейна для каждого
    /// 3. Фильтрует по порогу (distance <= max(len/2, 3))
    /// 4. Сортирует по расстоянию (меньше = лучше)
    /// 5. Возвращает топ-N результатов
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::repository::TypeRepository;
    /// # use bsl_shared::domain::types::MetadataKind;
    /// # use std::sync::Arc;
    /// # let repository: Arc<dyn TypeRepository> = todo!();
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// // Опечатка: "Контрогенты" вместо "Контрагенты"
    /// let suggestions = lookup.suggest_similar_names(
    ///     MetadataKind::Catalog,
    ///     "Контрогенты",
    ///     3
    /// );
    /// // -> ["Контрагенты"]
    /// # let _ = suggestions;
    /// ```
    pub fn suggest_similar_names(
        &self,
        kind: MetadataKind,
        name: &str,
        max_suggestions: usize,
    ) -> Vec<String> {
        let all_objects = self.repository.get_metadata_objects_by_kind(kind);

        let mut candidates: Vec<(String, usize)> = all_objects
            .into_iter()
            .filter_map(|obj_name| {
                let distance = levenshtein_distance(name, &obj_name);
                // Порог: до половины длины имени, но минимум 3
                let threshold = (name.chars().count() / 2).max(3);
                if distance <= threshold {
                    Some((obj_name, distance))
                } else {
                    None
                }
            })
            .collect();

        // Сортируем по расстоянию (меньше = лучше совпадение)
        candidates.sort_by_key(|(_, dist)| *dist);

        candidates
            .into_iter()
            .take(max_suggestions)
            .map(|(n, _)| n)
            .collect()
    }

    /// Проверяет, загружена ли конфигурация в репозиторий
    ///
    /// # Возвращает
    ///
    /// `true` если есть хотя бы один конфигурационный тип
    ///
    /// # Использование
    ///
    /// Полезно для определения, нужно ли выполнять валидацию
    /// объектов метаданных или просто пропустить проверку.
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::repository::TypeRepository;
    /// # use bsl_shared::domain::types::MetadataKind;
    /// # use std::sync::Arc;
    /// # let repository: Arc<dyn TypeRepository> = todo!();
    /// # let kind = MetadataKind::Catalog;
    /// # let name = "Контрагенты";
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// if lookup.is_configuration_loaded() {
    ///     // Выполняем валидацию объектов метаданных
    ///     if !lookup.exists_metadata_object(kind, name) {
    ///         // Генерируем ошибку
    ///     }
    /// } else {
    ///     // Пропускаем валидацию - конфигурация не загружена
    /// }
    /// ```
    pub fn is_configuration_loaded(&self) -> bool {
        let stats = self.repository.get_stats();
        stats.configuration_types > 0
    }

    /// Возвращает type completion labels из repository-backed contract.
    ///
    /// Semantic completion v2 использует этот список на canonical path вместо
    /// discovery/search `IndexSnapshot`.
    pub fn get_completion_type_names(&self) -> Vec<String> {
        self.repository
            .get_all_types()
            .into_iter()
            .filter(|raw| !matches!(raw.source, RawDataSource::Configuration))
            .map(|raw| raw.name)
            .collect()
    }

    /// Возвращает глобальные функции из repository-backed signature contract.
    pub fn get_global_function_names(&self) -> Vec<String> {
        let mut names = self
            .repository
            .get_signature_index_clone()
            .get_global_functions()
            .keys()
            .map(|name| name.display().to_string())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        names
    }
}
