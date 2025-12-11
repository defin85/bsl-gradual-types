//! Context-Aware Variable Resolution
//!
//! Direction 2: Generic Collections Inference - Integration

use super::type_resolver::TypeResolver;
use crate::domain::types::TypeResolution;

impl TypeResolver {
    /// Резолюция переменной с использованием SymbolTable контекста
    ///
    /// Используется для вывода Generic типов из flow-sensitive анализа.
    pub fn resolve_variable_with_context(
        &self,
        var_name: &str,
        symbol_table: &crate::ir::SymbolTable,
        scope_id: crate::ir::ScopeId,
    ) -> TypeResolution {
        use tracing::{debug, info};

        // Ищем переменную в scope hierarchy
        if let Some(resolution) = symbol_table.get_variable_type(scope_id, var_name) {
            info!(
                "resolve_variable_with_context('{}', scope={:?}): TypeResolution = {:?}",
                var_name, scope_id, resolution
            );

            use crate::domain::types::{Certainty, ResolutionResult};

            match (&resolution.certainty, &resolution.result) {
                (Certainty::Unknown, _) => {
                    debug!("  -> TypeResolution::unknown()");
                    return TypeResolution::unknown();
                }
                (_, ResolutionResult::Generic(gen)) => {
                    let type_params: Vec<String> = gen
                        .type_params
                        .iter()
                        .map(|ct| {
                            let temp = TypeResolution::known(ct.clone());
                            temp.type_name()
                        })
                        .collect();
                    let certainty = match resolution.certainty {
                        Certainty::Known => 1.0,
                        Certainty::Inferred(c) => c,
                        Certainty::Unknown => 0.0,
                    };
                    debug!(
                        "  -> Generic: base={}, params={:?}",
                        gen.base_type, type_params
                    );
                    return self.resolve_generic_from_hint(&gen.base_type, &type_params, certainty);
                }
                _ => {
                    let type_name = resolution.type_name();
                    info!("  -> Resolving type_name: '{}'", type_name);
                    let resolved = self.resolve_expression_sync(&type_name);
                    info!("  -> Resolution result: {:?}", resolved.result);
                    return resolved;
                }
            }
        }

        info!(
            "resolve_variable_with_context('{}', scope={:?}): NOT FOUND in SymbolTable",
            var_name, scope_id
        );
        TypeResolution::unknown()
    }

    /// Резолюция Generic типа из TypeResolution
    pub(crate) fn resolve_generic_from_hint(
        &self,
        base_type: &str,
        type_params: &[String],
        certainty: f32,
    ) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, FacetKind, GenericType, ResolutionResult, ResolutionSource,
        };

        // Конвертируем строки типов в ConcreteType
        let concrete_params: Vec<ConcreteType> = type_params
            .iter()
            .filter(|p| *p != "?")
            .filter_map(|p| {
                let resolved = self.resolve_expression_sync(p);
                match resolved.result {
                    ResolutionResult::Concrete(ct) => Some(ct),
                    _ => None,
                }
            })
            .collect();

        // Если после фильтрации не осталось параметров — возвращаем базовый тип без Generic
        if concrete_params.is_empty() {
            return self.resolve_expression_sync(base_type);
        }

        // Создаём GenericType
        let generic_type = GenericType {
            base_type: base_type.to_string(),
            type_params: concrete_params,
        };

        // Определяем уровень certainty
        let certainty_level = if certainty > 0.9 {
            Certainty::Known
        } else if certainty > 0.5 {
            Certainty::Inferred(certainty)
        } else {
            Certainty::Inferred(0.5)
        };

        TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: certainty_level,
            source: ResolutionSource::Inferred,
            metadata: crate::domain::types::ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!(
                    "Generic type inferred from flow-sensitive analysis (certainty: {:.0}%)",
                    certainty * 100.0
                )],
                uncertainty_reason: None,
            },
            active_facet: Some(FacetKind::Collection),
            available_facets: vec![FacetKind::Collection],
        }
    }
}
