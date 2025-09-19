//! The core analysis engine, independent of any specific adapter (backend, CLI, etc.).

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::resolver::TypeResolver;
use crate::domain::types::TypeResolution;
use crate::loaders::{
    ConfigurationGuidedParser, SyntaxHelperParser, 
    convert_syntax_helper_to_raw
};

/// A simplified, self-contained analysis result for the CLI.
#[derive(Debug, Clone)]
pub struct CliAnalysisResult {
    pub file_path: String,
    pub type_resolutions: Vec<(String, TypeResolution)>,
    pub analysis_duration_ms: u128,
}

/// The core analysis engine.
/// It orchestrates parsing and type resolution.
pub struct AnalysisEngine {
    resolver: Arc<TypeResolver>,
}

impl AnalysisEngine {
    /// Creates a new analysis engine.
    pub fn new(syntax_helper_path: &str, config_path: &str) -> Result<Self> {
        
        let mut syntax_parser = SyntaxHelperParser::new();
        let _config_parser = ConfigurationGuidedParser::new(config_path);

        // Run parsers to load data
        if Path::new(syntax_helper_path).exists() {
            syntax_parser.parse_directory(syntax_helper_path)?;
        }
        
        // This parser needs to be implemented to return Vec<DiscoveredMetadata>
        // let discovered_metadata = if Path::new(config_path).exists() {
        //     config_parser.parse_with_configuration_guide()?
        // } else {
        //     vec![]
        // };

        let repository = Arc::new(InMemoryTypeRepository::new());
        
        // Convert and load platform types
        let platform_raw_data = convert_syntax_helper_to_raw(&syntax_parser.export_database());
        repository.load_types(platform_raw_data)?;
        
        // Convert and load configuration types
        // let config_raw_data = convert_discovered_metadata_to_raw(&discovered_metadata);
        // repository.load_types(config_raw_data)?;

        let resolver = Arc::new(TypeResolver::new(repository));

        Ok(Self { resolver })
    }

    /// Analyzes a single BSL file.
    pub async fn analyze_file<P: AsRef<Path>>(&self, path: P) -> Result<CliAnalysisResult> {
        let start_time = std::time::Instant::now();
        let path_str = path.as_ref().display().to_string();

        let _content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path_str, e))?;

        // MOCK IMPLEMENTATION
        let mut resolutions_map = HashMap::new();
        resolutions_map.insert(
            "ПеременнаяА".to_string(),
            self.resolver.resolve_expression_async("Строка").await,
        );
        resolutions_map.insert(
            "ПеременнаяБ".to_string(),
            self.resolver.resolve_expression_async("Справочники.Контрагенты").await,
        );
        let resolutions: Vec<(String, TypeResolution)> = resolutions_map.into_iter().collect();

        let result = CliAnalysisResult {
            file_path: path_str,
            type_resolutions: resolutions,
            analysis_duration_ms: start_time.elapsed().as_millis(),
        };

        Ok(result)
    }
}