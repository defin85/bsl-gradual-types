//! Runtime contracts for gradual typing

use super::types::{Certainty, ConcreteType, Contract, ResolutionResult, TypeResolution};

/// Contract generator for runtime type checking
pub struct ContractGenerator {
    threshold: f32,
    mode: ContractMode,
}

/// Contract generation mode
#[derive(Debug, Clone, Copy)]
pub enum ContractMode {
    /// Only generate warnings
    Warning,
    /// Add runtime assertions
    Assert,
    /// Log violations
    Report,
}

impl ContractGenerator {
    pub fn new(threshold: f32, mode: ContractMode) -> Self {
        Self { threshold, mode }
    }

    /// Generate contract for uncertain type resolution
    pub fn generate_contract(&self, resolution: &TypeResolution) -> Option<Contract> {
        match resolution.certainty {
            Certainty::Inferred(confidence) if confidence < self.threshold => {
                Some(self.create_runtime_check(resolution))
            }
            Certainty::Unknown => Some(self.create_dynamic_check(resolution)),
            _ => None,
        }
    }

    fn create_runtime_check(&self, resolution: &TypeResolution) -> Contract {
        Contract {
            check_code: self.generate_check_code(resolution),
            error_message: format!(
                "Type mismatch: expected {:?}, confidence: low",
                resolution.result
            ),
        }
    }

    fn create_dynamic_check(&self, resolution: &TypeResolution) -> Contract {
        // Генерируем универсальную проверку для неизвестного типа
        let check_code = match self.mode {
            ContractMode::Warning => {
                "// ВНИМАНИЕ: Тип не определён статически, требуется runtime проверка".to_string()
            }
            ContractMode::Assert => r#"// Runtime контракт для неизвестного типа
                Если НЕ ЗначениеЗаполнено(Значение) Тогда
                    ВызватьИсключение "Ошибка типа: значение не заполнено";
                КонецЕсли;"#
                .to_string(),
            ContractMode::Report => r#"// Логирование типа для анализа
                ЗаписьЖурналаРегистрации("ГрадуальнаяТипизация", 
                    УровеньЖурналаРегистрации.Информация,
                    "НеизвестныйТип", 
                    Строка(ТипЗнч(Значение)));
                "#
            .to_string(),
        };

        Contract {
            check_code,
            error_message: format!(
                "Тип не может быть определён статически для: {:?}",
                resolution.metadata.file
            ),
        }
    }

    fn generate_check_code(&self, resolution: &TypeResolution) -> String {
        match &resolution.result {
            ResolutionResult::Concrete(concrete_type) => {
                self.generate_concrete_check(concrete_type)
            }
            ResolutionResult::Union(weighted_types) => {
                // Преобразуем WeightedType в ConcreteType
                let concrete_types: Vec<ConcreteType> =
                    weighted_types.iter().map(|wt| wt.type_.clone()).collect();
                self.generate_union_check(&concrete_types)
            }
            ResolutionResult::Conditional(condition) => self.generate_conditional_check(condition),
            ResolutionResult::Dynamic => self.generate_dynamic_check_code(),
            ResolutionResult::Contextual(contextual) => {
                format!(
                    "// Контекстный тип: {:?} в контексте {:?}",
                    contextual.base_type, contextual.context
                )
            }
        }
    }

    fn generate_concrete_check(&self, concrete_type: &ConcreteType) -> String {
        match concrete_type {
            ConcreteType::Platform(platform) => {
                let type_check = format!("Тип(\"{}\")", platform.name);
                self.format_check(&type_check, &platform.name)
            }
            ConcreteType::Configuration(config) => {
                let type_name = match config.kind {
                    super::types::MetadataKind::Catalog => {
                        format!("СправочникСсылка.{}", config.name)
                    }
                    super::types::MetadataKind::Document => {
                        format!("ДокументСсылка.{}", config.name)
                    }
                    super::types::MetadataKind::Enum => {
                        format!("ПеречислениеСсылка.{}", config.name)
                    }
                    super::types::MetadataKind::Register => {
                        format!("РегистрСведенийКлючЗаписи.{}", config.name)
                    }
                    _ => config.name.clone(),
                };
                let type_check = format!("Тип(\"{}\")", type_name);
                self.format_check(&type_check, &type_name)
            }
            ConcreteType::Primitive(primitive) => {
                let type_name = format!("{:?}", primitive);
                let type_check = format!("Тип(\"{}\")", type_name);
                self.format_check(&type_check, &type_name)
            }
            ConcreteType::Special(special) => {
                let type_name = format!("{:?}", special);
                let type_check = format!("Тип(\"{}\")", type_name);
                self.format_check(&type_check, &type_name)
            }
            ConcreteType::GlobalFunction(func) => {
                // Глобальные функции не могут быть значениями переменных в BSL
                // Это ошибка типов, но генерируем проверку для полноты
                let error_msg =
                    format!("Глобальная функция '{}' не может быть значением", func.name);
                format!("ВызватьИсключение(\"{}\")", error_msg)
            }
        }
    }

    fn generate_union_check(&self, types: &[ConcreteType]) -> String {
        let type_names: Vec<String> = types
            .iter()
            .map(|t| match t {
                ConcreteType::Platform(p) => p.name.clone(),
                ConcreteType::Configuration(c) => format!("{:?}.{}", c.kind, c.name),
                ConcreteType::Primitive(p) => format!("{:?}", p),
                ConcreteType::Special(s) => format!("{:?}", s),
                ConcreteType::GlobalFunction(f) => format!("GlobalFunction.{}", f.name),
            })
            .collect();

        let checks = type_names
            .iter()
            .map(|name| format!("ТипЗнч(Значение) = Тип(\"{}\")", name))
            .collect::<Vec<_>>()
            .join(" ИЛИ ");

        match self.mode {
            ContractMode::Assert => {
                format!(
                    r#"// Проверка составного типа
                Если НЕ ({}) Тогда
                    ВызватьИсключение "Ошибка типа: ожидался один из типов {}";
                КонецЕсли;"#,
                    checks,
                    type_names.join(", ")
                )
            }
            _ => format!("// Составной тип: {}", type_names.join(" | ")),
        }
    }

    fn generate_conditional_check(&self, condition: &super::types::ConditionalType) -> String {
        format!("// Условный тип: {}", condition.condition)
    }

    fn generate_dynamic_check_code(&self) -> String {
        "// Динамический тип - проверка в runtime".to_string()
    }

    fn format_check(&self, type_check: &str, type_name: &str) -> String {
        match self.mode {
            ContractMode::Warning => {
                format!("// ВНИМАНИЕ: Проверьте тип {}", type_name)
            }
            ContractMode::Assert => {
                format!(
                    r#"// Runtime контракт для типа {}
                Если ТипЗнч(Значение) <> {} Тогда
                    ВызватьИсключение "Ошибка типа: ожидался {}, получен " + Строка(ТипЗнч(Значение));
                КонецЕсли;"#,
                    type_name, type_check, type_name
                )
            }
            ContractMode::Report => {
                format!(
                    r#"// Логирование проверки типа
                Если ТипЗнч(Значение) <> {} Тогда
                    ЗаписьЖурналаРегистрации("ТипМисматч", 
                        УровеньЖурналаРегистрации.Предупреждение,
                        "{}", 
                        "Ожидался {}, получен " + Строка(ТипЗнч(Значение)));
                КонецЕсли;"#,
                    type_check, type_name, type_name
                )
            }
        }
    }
}
