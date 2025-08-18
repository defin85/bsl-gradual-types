//! Context handling for type resolution

use super::types::{FacetKind, ExecutionContext};

/// Context for type resolution
#[derive(Debug, Clone)]
pub struct Context {
    /// Current execution context
    pub execution: ExecutionContext,
    
    /// Parent expression context
    pub parent: Option<String>,
    
    /// Method being called
    pub method: Option<String>,
    
    /// Current scope variables
    pub scope: Vec<ScopeVariable>,
}

/// Variable in current scope
#[derive(Debug, Clone)]
pub struct ScopeVariable {
    pub name: String,
    pub type_hint: Option<String>,
}

/// Context resolver for determining active facets
pub struct ContextResolver;

impl ContextResolver {
    /// Determine which facet is active in given context
    pub fn determine_facet(&self, context: &Context) -> FacetKind {
        if let Some(parent) = &context.parent {
            if parent.contains("Справочники") || parent.contains("Catalogs") {
                return FacetKind::Manager;
            }
            if parent.contains("Метаданные") || parent.contains("Metadata") {
                return FacetKind::Metadata;
            }
        }
        
        if let Some(method) = &context.method {
            if method == "СоздатьЭлемент" || method == "CreateItem" {
                return FacetKind::Object;
            }
            if method == "НайтиПоКоду" || method == "FindByCode" {
                return FacetKind::Reference;
            }
        }
        
        // Default to reference facet
        FacetKind::Reference
    }
    
    /// Check if context allows certain operations
    pub fn is_allowed(&self, context: &Context, operation: &str) -> bool {
        match context.execution {
            ExecutionContext::Server => {
                // Server context has full access
                true
            }
            ExecutionContext::Client | ExecutionContext::WebClient => {
                // Client contexts have limited access
                !operation.contains("Database")
            }
            _ => true,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            execution: ExecutionContext::Server,
            parent: None,
            method: None,
            scope: Vec::new(),
        }
    }
    
    pub fn with_parent(mut self, parent: String) -> Self {
        self.parent = Some(parent);
        self
    }
    
    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
}