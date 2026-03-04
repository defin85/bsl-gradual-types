use super::*;

impl ConfigurationDiscovery {
    /// Автоматически обнаруживает папку с конфигурацией.
    ///
    /// Если `Configuration.xml` находится прямо в `base_path` - возвращает `base_path`.
    /// Иначе сканирует подпапки и возвращает первую подпапку с `Configuration.xml`.
    #[allow(dead_code)] // Вспомогательный метод для будущего использования.
    fn find_configuration_folder(&self) -> Result<PathBuf> {
        // Сначала проверяем прямо в base_path (обратная совместимость).
        let direct_config = self.base_path.join("Configuration.xml");
        if direct_config.exists() {
            tracing::debug!(
                "✅ Configuration.xml найден напрямую в {:?}",
                self.base_path
            );
            return Ok(self.base_path.clone());
        }

        // Если не найден - сканируем подпапки.
        tracing::debug!(
            "🔍 Сканирование подпапок в {:?} для поиска конфигурации...",
            self.base_path
        );

        let entries = fs::read_dir(&self.base_path).map_err(|e| {
            format!(
                "Не удалось прочитать директорию {:?}: {}",
                self.base_path, e
            )
        })?;

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir_path = entry.path();
                    let config_in_subdir = subdir_path.join("Configuration.xml");

                    if config_in_subdir.exists() {
                        tracing::debug!("✅ Найдена конфигурация в подпапке: {:?}", subdir_path);
                        return Ok(subdir_path);
                    }
                }
            }
        }

        Err(format!(
            "Configuration.xml не найден ни в {:?}, ни в подпапках. \
            Убедитесь, что указан правильный путь к выгруженной конфигурации.",
            self.base_path
        )
        .into())
    }

    /// Обнаруживает модули объекта (ObjectModule, ManagerModule, RecordSetModule)
    ///
    /// Проверяет наличие модулей в папке Ext объекта метаданных.
    ///
    /// # Параметры
    ///
    /// - `object_type` - тип объекта в множественном числе ("Documents", "Catalogs")
    /// - `object_name` - имя объекта ("ЗаказНаряды", "Контрагенты")
    ///
    /// # Возвращает
    ///
    /// Кортеж из трёх опциональных PathBuf:
    /// - ObjectModule.bsl
    /// - ManagerModule.bsl
    /// - RecordSetModule.bsl
    ///
    /// # Examples
    ///
    /// ```text
    /// let (object_mod, manager_mod, record_set_mod) =
    ///     discovery.discover_object_modules("Catalogs", "Контрагенты");
    /// assert!(object_mod.is_some());
    /// ```
    pub fn discover_object_modules(
        &self,
        object_type: &str, // "Documents", "Catalogs"
        object_name: &str, // "ЗаказНаряды", "Контрагенты"
    ) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
        let base_dir = self
            .base_path
            .join(object_type)
            .join(object_name)
            .join("Ext");

        let object_module = base_dir.join("ObjectModule.bsl");
        let manager_module = base_dir.join("ManagerModule.bsl");
        let record_set_module = base_dir.join("RecordSetModule.bsl");

        tracing::trace!(
            "🔍 Scanning for modules in {:?} (object_type: {}, object_name: {})",
            base_dir,
            object_type,
            object_name
        );

        (
            if object_module.exists() {
                tracing::trace!("  ✅ Found ObjectModule.bsl");
                Some(object_module)
            } else {
                None
            },
            if manager_module.exists() {
                tracing::trace!("  ✅ Found ManagerModule.bsl");
                Some(manager_module)
            } else {
                None
            },
            if record_set_module.exists() {
                tracing::trace!("  ✅ Found RecordSetModule.bsl");
                Some(record_set_module)
            } else {
                None
            },
        )
    }

    pub fn discover_common_module_path(&self, object_name: &str) -> Option<PathBuf> {
        let module_path = self
            .base_path
            .join("CommonModules")
            .join(object_name)
            .join("Ext")
            .join("Module.bsl");

        if module_path.exists() {
            Some(module_path)
        } else {
            None
        }
    }

    /// Обнаруживает предопределённые элементы из `Ext/Predefined.xml`
    pub fn discover_predefined_items(
        &self,
        object_type: &str, // "Catalogs", "ChartsOfAccounts", ...
        object_name: &str, // имя объекта метаданных
    ) -> Vec<String> {
        let predefined_xml = self
            .base_path
            .join(object_type)
            .join(object_name)
            .join("Ext")
            .join("Predefined.xml");

        if !predefined_xml.exists() {
            return Vec::new();
        }

        match UniversalMetadataParser::parse_predefined_items(&predefined_xml) {
            Ok(items) => items,
            Err(e) => {
                warn!(
                    "⚠️ Failed to parse predefined items for {}.{}: {}",
                    object_type, object_name, e
                );
                Vec::new()
            }
        }
    }

    /// Обнаруживает формы для объекта метаданных (рекомендуемый метод)
    ///
    /// Принимает корректный XML-kind объекта (`Document`, `Catalog`,
    /// `BusinessProcess`, ...), чтобы правильно формировать `owner_type` для FormParser.
    ///
    /// # Параметры
    /// - `folder_name` - папка в конфигурации ("Documents", "Catalogs", "BusinessProcesses", ...)
    /// - `object_type` - XML-kind ("Document", "Catalog", "BusinessProcess", ...)
    /// - `object_name` - имя объекта ("ЗаказНаряды", "Контрагенты")
    pub fn discover_forms_for_object(
        &self,
        folder_name: &str,
        object_type: &str,
        object_name: &str,
    ) -> Result<Vec<FormMetadata>> {
        let forms_dir = self
            .base_path
            .join(folder_name)
            .join(object_name)
            .join("Forms");

        if !forms_dir.exists() {
            tracing::trace!("  ℹ️ Forms directory not found: {:?}", forms_dir);
            return Ok(Vec::new());
        }

        let mut forms = Vec::new();

        for entry in fs::read_dir(&forms_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let form_name = entry.file_name().to_string_lossy().to_string();
            let form_xml = entry.path().join("Ext").join("Form.xml");

            if form_xml.exists() {
                let owner_type = format!("{}.{}", object_type, object_name);

                match FormParser::parse_form_xml(&form_xml, &owner_type, &form_name) {
                    Ok(form) => {
                        tracing::trace!("    ✅ Parsed form: {}", form_name);
                        forms.push(form);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse form {}: {}", form_name, e);
                    }
                }
            }
        }

        if !forms.is_empty() {
            tracing::debug!(
                "📋 Discovered {} forms for {}.{}",
                forms.len(),
                object_type,
                object_name
            );
        }

        Ok(forms)
    }
}
