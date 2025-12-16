//! Обработка Generic типов с подстановкой типовых параметров.
//!
//! Generic типы позволяют описывать параметризованные коллекции,
//! например ТабличнаяЧасть<СтрокаРаботы>.

use super::TypeMetadataLookup;
use crate::domain::types::{ConcreteType, GenericType, RawMethodData};

impl TypeMetadataLookup {
    /// Возвращает методы для Generic типа с подстановкой типовых параметров
    ///
    /// # Примеры
    /// ```ignore
    /// Generic: ТабличнаяЧасть<СтрокаРаботы>
    /// Метод: Добавить() -> T
    /// Результат: Добавить() -> СтрокаРаботы
    /// ```
    pub fn get_methods_for_generic(&self, generic_type: &GenericType) -> Vec<RawMethodData> {
        tracing::debug!(
            "Получение методов для Generic типа: {}",
            generic_type.base_type
        );

        // 1. Получаем методы базового типа (например, "ТабличнаяЧасть")
        let base_raw = self.repository.find_type(&generic_type.base_type);
        let base_methods = base_raw
            .as_ref()
            .map(|raw| raw.methods.clone())
            .unwrap_or_default();
        let collection_item_type = base_raw
            .as_ref()
            .and_then(|raw| raw.collection_item_type.clone());

        tracing::trace!("  Найдено {} методов базового типа", base_methods.len());

        // 2. Если есть типовой параметр (например, СтрокаРаботы)
        if let Some(param_type) = generic_type.type_params.first() {
            // Форматируем имя типового параметра
            let param_type_name = self.format_concrete_type(param_type);

            tracing::trace!("  Подстановка типового параметра: T -> {}", param_type_name);

            // 3. Подставляем конкретный тип вместо "T" в методах
            base_methods
                .into_iter()
                .map(|mut method| {
                    // Подменяем "T" на конкретный тип в return_type
                    if method.return_type == "T" {
                        method.return_type = param_type_name.clone();
                        tracing::trace!(
                            "    Метод {}: return_type T -> {}",
                            method.name,
                            param_type_name
                        );
                    }

                    // Доп. правило: если возвращается "элемент коллекции" без параметра,
                    // параметризуем его: ItemType -> ItemType<T>
                    if let Some(ref item_type) = collection_item_type {
                        if method.return_type == *item_type {
                            method.return_type = format!("{}<{}>", item_type, param_type_name);
                        }
                    }

                    // Подменяем "T" в типах параметров
                    for param in &mut method.params {
                        if param.param_type == "T" {
                            param.param_type = param_type_name.clone();
                            tracing::trace!(
                                "      Параметр {}: тип T -> {}",
                                param.name,
                                param_type_name
                            );
                        }
                    }

                    method
                })
                .collect()
        } else {
            // Нет типовых параметров -> возвращаем методы как есть
            tracing::warn!(
                "  Generic тип {} не имеет параметров",
                generic_type.base_type
            );
            base_methods
        }
    }

    /// Форматирует ConcreteType в строку для отображения
    ///
    /// # Примеры
    /// - `Platform(Строка)` -> `"Строка"`
    /// - `Configuration(Справочники.Контрагенты)` -> `"Справочники.Контрагенты"`
    /// - `TabularRow(СтрокаРаботы)` -> `"СтрокаРаботы"`
    pub(crate) fn format_concrete_type(&self, concrete: &ConcreteType) -> String {
        match concrete {
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => {
                // Формируем полное имя: "Справочники.Контрагенты"
                format!("{}.{}", ct.kind.to_prefix(), ct.name)
            }
            ConcreteType::Primitive(prim) => format!("{:?}", prim),
            ConcreteType::Special(spec) => format!("{:?}", spec),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
        }
    }
}
