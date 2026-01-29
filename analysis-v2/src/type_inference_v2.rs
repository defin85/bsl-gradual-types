use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{Certainty, TypeResolution, UncertaintyReason};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_syntax::ast::{Expression, Program, Statement};

use crate::ast_to_ir::{is_global_collection, lookup_global_collection};
use crate::SemanticDeps;

#[derive(Debug, Clone)]
pub(crate) struct TypeIndexEntry {
    pub span: bsl_shared::ir::Span,
    pub resolution: TypeResolution,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TypeIndex {
    entries: Vec<TypeIndexEntry>,
}

impl TypeIndex {
    pub(crate) fn type_at_position(&self, line: u32, column: u32) -> Option<TypeResolution> {
        self.entries
            .iter()
            .filter(|entry| entry.span.contains(line, column))
            .min_by_key(|entry| {
                let lines = entry.span.end_line.saturating_sub(entry.span.start_line);
                let cols = if lines == 0 {
                    entry.span.end_column.saturating_sub(entry.span.start_column)
                } else {
                    lines * 1000
                };
                lines * 1000 + cols
            })
            .map(|entry| entry.resolution.clone())
    }
}

#[derive(Default, Clone)]
struct TypeEnv {
    variables: HashMap<String, TypeResolution>,
}

struct TypeInferencer {
    deps: Arc<SemanticDeps>,
    resolver: Arc<TypeResolver>,
    signature_index: SignatureIndex,
    metadata_lookup: TypeMetadataLookup,
}

impl TypeInferencer {
    fn new(deps: Arc<SemanticDeps>) -> Self {
        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
        let signature_index = deps.signature_index.clone();
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
        Self {
            deps,
            resolver,
            signature_index,
            metadata_lookup,
        }
    }

    fn build_index(&self, program: &Program, file_path: &str) -> TypeIndex {
        let mut env = TypeEnv::default();
        let mut index = TypeIndex::default();
        self.seed_module_context(file_path, &mut env);
        for stmt in &program.statements {
            self.visit_statement(stmt, &mut env, &mut index);
        }
        index
    }

    fn seed_module_context(&self, file_path: &str, env: &mut TypeEnv) {
        let path = Path::new(file_path);
        let Ok(location) = CodeLocation::determine_from_path(path) else {
            return;
        };

        let ModuleType::FormModule {
            form_name,
            owner_type,
        } = location.module_type
        else {
            return;
        };

        let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
            return;
        };

        let Some(kind) = MetadataKind::from_xml_tag(xml_kind) else {
            return;
        };

        let collection = kind.display_name();
        let form_type_name = format!("Формы.{}.{}.{}", collection, object_name, form_name);

        if self.deps.repository.find_type(&form_type_name).is_none() {
            return;
        }

        let form_elements_type_name =
            format!("ЭлементыФормы.{}.{}.{}", collection, object_name, form_name);
        if self.deps.repository.find_type(&form_elements_type_name).is_some() {
            env.variables.insert(
                "Элементы".to_lowercase(),
                TypeResolution::explicit(&form_elements_type_name),
            );
        }

        env.variables.insert(
            "ЭтаФорма".to_lowercase(),
            TypeResolution::explicit(&form_type_name),
        );

        let Some(form_type) = self.deps.repository.find_type(&form_type_name) else {
            return;
        };

        for prop in form_type.properties {
            if prop.name.to_lowercase() == "элементы" {
                continue;
            }
            if prop.prop_type.is_empty() {
                continue;
            }
            if prop.prop_type.contains("cfg:") {
                env.variables
                    .insert(prop.name.to_lowercase(), TypeResolution::inferred(&prop.prop_type));
                continue;
            }
            if is_configuration_type_pattern(&prop.prop_type) {
                let resolved = self.resolver.resolve_expression_sync(&prop.prop_type);
                let resolved = if resolved.is_unknown() {
                    TypeResolution::inferred(&prop.prop_type)
                } else {
                    resolved
                };
                env.variables.insert(prop.name.to_lowercase(), resolved);
                continue;
            }
            let resolved = if self.deps.repository.find_type(&prop.prop_type).is_some() {
                TypeResolution::explicit(&prop.prop_type)
            } else {
                self.try_resolve_configuration_type(&prop.prop_type)
                    .unwrap_or_else(|| self.resolver.resolve_expression_sync(&prop.prop_type))
            };
            env.variables.insert(prop.name.to_lowercase(), resolved);
        }
    }

    fn visit_statement(&self, stmt: &Statement, env: &mut TypeEnv, index: &mut TypeIndex) {
        match stmt {
            Statement::VarDeclaration {
                name, type_hint, ..
            } => {
                let resolution = type_hint
                    .as_deref()
                    .map(TypeResolution::explicit)
                    .unwrap_or_else(TypeResolution::unknown);
                env.variables.insert(name.to_lowercase(), resolution);
            }
            Statement::Assignment { target, value, .. } => {
                let value_type = self.infer_expr(value, env, index);
                if let Expression::Identifier { name, .. } = target {
                    let key = name.to_lowercase();
                    env.variables.insert(key.clone(), value_type);
                    // Hover/type-at-position на имени переменной после присваивания
                    // должен видеть новый тип.
                    self.record(expr_span(target), env.variables[&key].clone(), index);
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let mut then_env = env.clone();
                for stmt in then_body {
                    self.visit_statement(stmt, &mut then_env, index);
                }
                if let Some(else_body) = else_body {
                    let mut else_env = env.clone();
                    for stmt in else_body {
                        self.visit_statement(stmt, &mut else_env, index);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                let _ = self.infer_expr(condition, env, index);
                let mut body_env = env.clone();
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::For {
                variable,
                start,
                end,
                body,
                ..
            } => {
                let _ = self.infer_expr(start, env, index);
                let _ = self.infer_expr(end, env, index);
                let mut body_env = env.clone();
                body_env
                    .variables
                    .insert(variable.to_lowercase(), TypeResolution::primitive("Число"));
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::ForEach {
                variable,
                collection,
                body,
                ..
            } => {
                let _ = self.infer_expr(collection, env, index);
                let mut body_env = env.clone();
                body_env
                    .variables
                    .insert(variable.to_lowercase(), TypeResolution::unknown());
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    let _ = self.infer_expr(value, env, index);
                }
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let mut try_env = env.clone();
                for stmt in try_body {
                    self.visit_statement(stmt, &mut try_env, index);
                }
                let mut except_env = env.clone();
                for stmt in except_body {
                    self.visit_statement(stmt, &mut except_env, index);
                }
            }
            Statement::Call { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::Execute { code, .. } => {
                let _ = self.infer_expr(code, env, index);
            }
            Statement::RaiseError { message, .. } => {
                if let Some(message) = message {
                    let _ = self.infer_expr(message, env, index);
                }
            }
            Statement::AddHandler { event, handler, .. }
            | Statement::RemoveHandler { event, handler, .. } => {
                let _ = self.infer_expr(event, env, index);
                let _ = self.infer_expr(handler, env, index);
            }
            Statement::Await { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::FunctionDecl { params, body, .. }
            | Statement::ProcedureDecl { params, body, .. } => {
                // TODO(v2): полноценное вычисление типов внутри функций на основе call graph.
                // Пока строим индекс внутри тела, наследуя module-level окружение
                // (например, implicit переменные модуля формы) и добавляя параметры.
                let mut fn_env = env.clone();
                for param in params {
                    fn_env
                        .variables
                        .insert(param.to_lowercase(), TypeResolution::unknown());
                }
                for stmt in body {
                    self.visit_statement(stmt, &mut fn_env, index);
                }
            }
            _ => {}
        }
    }

    fn record(&self, span: bsl_shared::ir::Span, resolution: TypeResolution, index: &mut TypeIndex) {
        index.entries.push(TypeIndexEntry { span, resolution });
    }

    fn infer_expr(
        &self,
        expr: &Expression,
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        let resolution = match expr {
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),
            Expression::Identifier { name, .. } => self.infer_identifier(name, env),
            Expression::New { type_name, args, .. } => {
                for arg in args {
                    let _ = self.infer_expr(arg, env, index);
                }
                self.infer_new_expression(type_name)
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let object_resolution = self.infer_expr(object, env, index);
                self.infer_property_access(&object_resolution, property)
            }
            Expression::Call { function, args, .. } => {
                for arg in args {
                    let _ = self.infer_expr(arg, env, index);
                }
                self.infer_call(function, env, index)
            }
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left_type = self.infer_expr(left, env, index);
                let right_type = self.infer_expr(right, env, index);
                self.infer_binary(operator, &left_type, &right_type)
            }
            Expression::Unary { operand, .. } => self.infer_expr(operand, env, index),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let then_type = self.infer_expr(then_expr, env, index);
                let else_type = self.infer_expr(else_expr, env, index);
                // TODO(v2): union типов.
                if then_type.type_name().eq_ignore_ascii_case(&else_type.type_name()) {
                    then_type
                } else {
                    TypeResolution::unknown()
                }
            }
            Expression::IndexAccess {
                object,
                index: index_expr,
                ..
            } => {
                let _ = self.infer_expr(object, env, index);
                let _ = self.infer_expr(index_expr, env, index);
                TypeResolution::unknown()
            }
            Expression::Await { expression, .. } => self.infer_expr(expression, env, index),
        };

        self.record(expr_span(expr), resolution.clone(), index);
        resolution
    }

    fn infer_identifier(&self, name: &str, env: &TypeEnv) -> TypeResolution {
        let name_lower = name.to_lowercase();
        if name_lower == "неопределено" || name_lower == "undefined" {
            return TypeResolution::primitive("Неопределено");
        }
        if name_lower == "null" {
            return TypeResolution::primitive("Null");
        }
        if matches!(name_lower.as_str(), "истина" | "ложь" | "true" | "false") {
            return TypeResolution::primitive("Булево");
        }

        if is_global_collection(name).is_some() {
            return TypeResolution::inferred(name);
        }

        if let Some(value) = env.variables.get(&name_lower) {
            return value.clone();
        }

        let common_module_type = format!("ОбщиеМодули.{}", name);
        if self.deps.repository.find_type(&common_module_type).is_some() {
            return self.resolver.resolve_expression_sync(&common_module_type);
        }

        TypeResolution::undeclared_variable(name)
    }

    fn infer_new_expression(&self, type_name: &str) -> TypeResolution {
        let clean = type_name.trim().trim_end_matches("()").trim();
        match clean {
            "Массив" => TypeResolution::generic("Массив", &["?"], Certainty::InferredWeak),
            "Соответствие" => {
                TypeResolution::generic("Соответствие", &["?", "?"], Certainty::InferredWeak)
            }
            "Список" => TypeResolution::generic("Список", &["?"], Certainty::InferredWeak),
            _ => {
                if self.deps.repository.find_type(clean).is_some() {
                    TypeResolution::explicit(clean)
                } else {
                    let mut res = TypeResolution::primitive(clean);
                    res.certainty = Certainty::Unknown;
                    res.metadata.uncertainty_reason = Some(UncertaintyReason::TypeNotFound {
                        name: clean.to_string(),
                    });
                    res
                }
            }
        }
    }

    fn infer_property_access(&self, object_type: &TypeResolution, property: &str) -> TypeResolution {
        let base_type = object_type.type_name();
        if let Some(info) = lookup_global_collection(&base_type) {
            // Справочники.Контрагенты -> СправочникМенеджер.Контрагенты
            let manager = format!("{}.{}", info.item_manager_type, property);
            return self.resolver.resolve_expression_sync(&manager);
        }

        let property_key = property.to_lowercase();
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
        if let Some(prop) = properties
            .into_iter()
            .find(|p| p.name.to_lowercase() == property_key)
        {
            if let Some(resolved) = self.try_resolve_configuration_type(&prop.prop_type) {
                return resolved;
            }
            if self.deps.repository.find_type(&prop.prop_type).is_some() {
                return self.resolver.resolve_expression_sync(&prop.prop_type);
            }
            // Типы свойств из metadata (в т.ч. синтетические UI-типы форм вроде "ГруппаФормы")
            // должны возвращаться даже если их документация не загружена в repository.
            return TypeResolution::inferred(&prop.prop_type);
        }

        TypeResolution::unknown()
    }

    fn infer_call(
        &self,
        function: &Expression,
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        match function {
            Expression::Identifier { name, .. } => self.infer_global_function_call(name),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let receiver = self.infer_expr(object, env, index);
                self.infer_method_call(&receiver, property)
            }
            _ => TypeResolution::unknown(),
        }
    }

    fn infer_global_function_call(&self, name: &str) -> TypeResolution {
        if let Some(sig) = self.signature_index.find_global_function(name) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                if let Some(resolved) = self.try_resolve_configuration_type(return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(return_type);
            }
        }
        TypeResolution::unknown()
    }

    fn infer_method_call(&self, receiver: &TypeResolution, method: &str) -> TypeResolution {
        let type_name = signature_lookup_type_name(receiver);
        if let Some(sig) = self.signature_index.find_method(&type_name, method) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                if let Some(resolved) = self.try_resolve_configuration_type(return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(return_type);
            }
        }

        let methods = self.metadata_lookup.get_methods(receiver);
        let method_key = method.to_lowercase();
        if let Some(m) = methods
            .into_iter()
            .find(|m| m.name.to_lowercase() == method_key)
        {
            if let Some(return_type) = (!m.return_type.is_empty()).then_some(m.return_type) {
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(&return_type);
            }
        }

        TypeResolution::unknown()
    }

    fn infer_binary(
        &self,
        operator: &str,
        left_type: &TypeResolution,
        right_type: &TypeResolution,
    ) -> TypeResolution {
        match operator {
            "+" => {
                if left_type.type_name().eq_ignore_ascii_case("Строка")
                    || right_type.type_name().eq_ignore_ascii_case("Строка")
                {
                    TypeResolution::primitive("Строка")
                } else {
                    TypeResolution::primitive("Число")
                }
            }
            "-" | "*" | "/" => TypeResolution::primitive("Число"),
            "=" | "<>" | ">" | "<" | ">=" | "<=" => TypeResolution::primitive("Булево"),
            _ => TypeResolution::unknown(),
        }
    }

    fn try_resolve_configuration_type(&self, type_name: &str) -> Option<TypeResolution> {
        if is_configuration_type_pattern(type_name) {
            return Some(self.resolver.resolve_expression_sync(type_name));
        }
        None
    }
}

fn expr_span(expr: &Expression) -> bsl_shared::ir::Span {
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

fn signature_lookup_type_name(resolution: &TypeResolution) -> String {
    let type_name = resolution.type_name();
    type_name
        .split('<')
        .next()
        .unwrap_or(type_name.as_str())
        .trim()
        .to_string()
}

pub(crate) fn build_type_index_with_path(
    program: &Program,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps).build_index(program, file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
    use bsl_shared::domain::type_id::TypeId;
    use bsl_shared::domain::types::{RawDataSource, RawPropertyData, RawTypeData};
    use bsl_syntax::ParseOptions;
    use bsl_shared::TypeRepository;

    fn parse(code: &str) -> Program {
        let parsed = bsl_syntax::parse(code, &ParseOptions::default()).expect("parse ok");
        parsed.program
    }

    fn deps_with_array_method() -> Arc<SemanticDeps> {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            }])
            .expect("load types");

        let mut sigs = SignatureIndex::new();
        sigs.add_platform_method(
            TypeId::new("Массив"),
            MethodSignature::new(
                "Количество".to_string(),
                Some("Массив".to_string()),
                vec![],
                Some("Число".to_string()),
                None,
                None,
                SignatureSource::Platform,
                None,
                Default::default(),
            ),
        );
        repository_impl.set_signature_index(sigs.clone());

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Arc::new(SemanticDeps {
            repository,
            signature_index: sigs,
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        })
    }

    #[test]
    fn builds_type_index_for_simple_assignment_and_method_call() {
        let program = parse(
            r#"Перем М;
М = Новый Массив();
Р = М.Количество();
"#,
        );
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let array_ident = index
            .type_at_position(1, 0)
            .expect("type at line 1 col 0");
        assert_eq!(array_ident.type_name(), "Массив<Неопределено>");

        let method_call = index
            .type_at_position(2, 6)
            .expect("type at method call");
        assert_eq!(method_call.type_name(), "Число");
    }

    #[test]
    fn seeds_form_module_context_for_elements_property_access() {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    ..Default::default()
                },
                RawTypeData {
                    name: "ЭлементыФормы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    properties: vec![RawPropertyData {
                        name: "СчетФактураПросмотр".to_string(),
                        prop_type: "ГруппаФормы".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ГруппаФормы".to_string(),
                    source: RawDataSource::Platform,
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        let deps = Arc::new(SemanticDeps {
            repository,
            signature_index: SignatureIndex::new(),
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        });

        let program = parse(
            r#"Процедура Тест()
    x = Элементы.СчетФактураПросмотр;
КонецПроцедуры
"#,
        );
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let loc = CodeLocation::determine_from_path(Path::new(file_path)).expect("code location");
        assert!(
            matches!(loc.module_type, ModuleType::FormModule { .. }),
            "expected FormModule for seed path, got {:?}",
            loc.module_type
        );
        assert!(
            repository_impl
                .find_type("Формы.Документы.Док1.Форма1")
                .is_some(),
            "expected synthetic form type to be present"
        );
        assert!(
            repository_impl
                .find_type("ЭлементыФормы.Документы.Док1.Форма1")
                .is_some(),
            "expected synthetic form elements type to be present"
        );

        let index = build_type_index_with_path(&program, file_path, deps);

        let receiver = index
            .type_at_position(1, 8)
            .expect("type at Элементы");
        assert_eq!(
            receiver.type_name(),
            "ЭлементыФормы.Документы.Док1.Форма1",
            "receiver should be seeded from form module context"
        );

        let member = index
            .type_at_position(1, 17)
            .expect("type at member access");
        assert_eq!(member.type_name(), "ГруппаФормы");
    }
}
