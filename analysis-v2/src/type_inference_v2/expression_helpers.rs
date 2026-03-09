use super::*;

impl TypeInferencer {
    pub(super) fn infer_binary(
        &self,
        operator: &str,
        left_type: &TypeResolution,
        right_type: &TypeResolution,
    ) -> TypeResolution {
        match operator {
            "+" => {
                let left_is_string = left_type.type_name().eq_ignore_ascii_case("Строка");
                let right_is_string = right_type.type_name().eq_ignore_ascii_case("Строка");

                if left_is_string && right_is_string {
                    return TypeResolution::primitive("Строка");
                }

                if left_is_string || right_is_string {
                    let mut res = TypeResolution::primitive("Строка");
                    res.certainty = Certainty::Unknown;
                    res.metadata.uncertainty_reason =
                        Some(UncertaintyReason::InvalidStringConcatenation {
                            left_type: left_type.type_name().to_string(),
                            right_type: right_type.type_name().to_string(),
                        });
                    return res;
                }

                TypeResolution::primitive("Число")
            }
            "-" | "*" | "/" => TypeResolution::primitive("Число"),
            "=" | "<>" | ">" | "<" | ">=" | "<=" => TypeResolution::primitive("Булево"),
            _ => TypeResolution::unknown(),
        }
    }

    pub(super) fn try_resolve_configuration_type(&self, type_name: &str) -> Option<TypeResolution> {
        if is_configuration_type_pattern(type_name) {
            return Some(self.resolver.resolve_expression_sync(type_name));
        }
        None
    }

    pub(super) fn resolve_property_type_by_name(
        &self,
        object_type: &TypeResolution,
        property_key: &str,
    ) -> Option<TypeResolution> {
        let properties = self.metadata_lookup.get_properties(object_type);
        let properties = if properties.is_empty() {
            self.deps
                .repository
                .find_type(&object_type.type_name())
                .map(|t| t.properties)
                .unwrap_or_default()
        } else {
            properties
        };
        let prop = properties
            .into_iter()
            .find(|p| p.name.to_lowercase() == property_key)?;

        if let Some(resolved) = self.try_resolve_configuration_type(&prop.prop_type) {
            return Some(resolved);
        }
        if self.deps.repository.find_type(&prop.prop_type).is_some() {
            return Some(self.resolver.resolve_expression_sync(&prop.prop_type));
        }
        // Типы свойств из metadata (в т.ч. синтетические UI-типы форм вроде "ГруппаФормы")
        // должны возвращаться даже если их документация не загружена в repository.
        Some(TypeResolution::inferred(&prop.prop_type))
    }
}

pub(super) fn expr_span(expr: &Expression) -> bsl_shared::ir::Span {
    match expr {
        Expression::Identifier { span, .. }
        | Expression::String { span, .. }
        | Expression::Number { span, .. }
        | Expression::Boolean { span, .. }
        | Expression::Date { span, .. }
        | Expression::Call { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Ternary { span, .. }
        | Expression::New { span, .. }
        | Expression::PropertyAccess { span, .. }
        | Expression::IndexAccess { span, .. }
        | Expression::Await { span, .. } => *span,
    }
}

pub(super) fn signature_lookup_type_name(resolution: &TypeResolution) -> String {
    let type_name = resolution.type_name();
    type_name
        .split('<')
        .next()
        .unwrap_or(type_name.as_str())
        .trim()
        .to_string()
}
