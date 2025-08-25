//! Загрузчик данных из синтакс-помощника
use crate::data::loaders::syntax_helper_parser::{
    GlobalFunctionInfo, SyntaxHelperParser, SyntaxNode, TypeInfo,
};
use crate::domain::types::{
    Certainty, ConcreteType, ExecutionContext, GlobalFunction, GlobalFunctionParameter, Method,
    PlatformType, Property, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use anyhow::Result;
use std::path::Path;
use super::TypeRepository;

pub struct SyntaxHelperLoader {
    type_repository: Box<dyn TypeRepository>,
}

impl SyntaxHelperLoader {
    pub fn new(type_repository: Box<dyn TypeRepository>) -> Self {
        Self { type_repository }
    }

    pub fn load_data(&mut self, syntax_helper_path: &Path) -> Result<()> {
        let mut parser = SyntaxHelperParser::new(); // Используем стандартные настройки
        parser.parse_directory(syntax_helper_path)?;
        let db = parser.export_database();

        for (_, node) in db.nodes.iter() {
            self.process_syntax_node(node);
        }

        Ok(())
    }

    fn process_syntax_node(&mut self, node: &SyntaxNode) {
        match node {
            SyntaxNode::Type(type_info) => {
                let resolution = self.convert_type_info_to_resolution(type_info);
                self.type_repository.add_resolution(resolution);
                println!("Processing Type: {}", type_info.identity.russian_name);
            }
            SyntaxNode::GlobalFunction(func_info) => {
                let resolution = self.convert_global_function_to_resolution(func_info);
                self.type_repository.add_resolution(resolution);
                println!("Processing Global Function: {}", func_info.name);
            }
            _ => {
                // Остальные узлы (категории, конструкторы, методы, свойства) пока не обрабатываем напрямую
            }
        }
    }

    fn convert_type_info_to_resolution(&self, type_info: &TypeInfo) -> TypeResolution {
        let methods: Vec<Method> = type_info
            .structure
            .methods
            .iter()
            .map(|method_name| {
                Method {
                    name: method_name.clone(),
                    parameters: Vec::new(), // TODO: загрузить параметры из базы данных
                    return_type: None,      // TODO: загрузить тип возврата
                    is_function: false,     // TODO: определить по типу возврата
                }
            })
            .collect();

        // Конвертируем свойства (только имена, полная информация недоступна)
        let properties: Vec<Property> = type_info
            .structure
            .properties
            .iter()
            .map(|property_name| {
                Property {
                    name: property_name.clone(),
                    type_: "Dynamic".to_string(), // TODO: получить тип из базы свойств
                    readonly: false,              // TODO: получить из базы свойств
                }
            })
            .collect();

        let platform_type = PlatformType {
            name: type_info.identity.russian_name.clone(),
            methods,
            properties,
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static, // From syntax helper
            metadata: ResolutionMetadata {
                notes: vec![type_info.documentation.type_description.clone()],
                ..Default::default()
            },
            active_facet: None,
            available_facets: type_info.metadata.available_facets.clone(),
        }
    }

    fn convert_global_function_to_resolution(
        &self,
        func_info: &GlobalFunctionInfo,
    ) -> TypeResolution {
        let parameters = func_info
            .parameters
            .iter()
            .map(|p| {
                GlobalFunctionParameter {
                    name: p.name.clone(),
                    type_: p
                        .type_name
                        .as_ref()
                        .map(|tn| Box::new(self.type_name_to_unknown_resolution(tn))),
                    is_optional: p.is_optional,
                    default_value: p.default_value.clone(),
                    description: None, // Not available in ParameterInfo
                }
            })
            .collect();

        let return_type = func_info
            .return_type
            .as_ref()
            .map(|rt| Box::new(self.type_name_to_unknown_resolution(rt)));

        let global_function = GlobalFunction {
            name: func_info.name.clone(),
            english_name: func_info.english_name.clone().unwrap_or_default(),
            parameters,
            return_type,
            pure: func_info.pure,
            polymorphic: false, // TODO: Determine if a function is polymorphic
            context_required: self.convert_availability_to_context(&func_info.contexts),
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::GlobalFunction(global_function)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                notes: vec![func_info.description.clone().unwrap_or_default()],
                ..Default::default()
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn type_name_to_unknown_resolution(&self, type_name: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                notes: vec![format!("Type to resolve: {}", type_name)],
                ..Default::default()
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn convert_availability_to_context(&self, availability: &[String]) -> Vec<ExecutionContext> {
        availability
            .iter()
            .map(|s| {
                match s.to_lowercase().trim() {
                    "сервер" | "server" => ExecutionContext::Server,
                    "тонкий клиент" | "thickclient" => ExecutionContext::ThickClient,
                    "веб-клиент" | "webclient" => ExecutionContext::WebClient,
                    "мобильный клиент" | "mobileclient" => {
                        ExecutionContext::MobileClient
                    }
                    "внешнее соединение" | "externalconnection" => {
                        ExecutionContext::ExternalConnection
                    }
                    "клиент" | "client" => ExecutionContext::Client,
                    _ => ExecutionContext::Server, // Default to Server for unknown contexts
                }
            })
            .collect()
    }
}
