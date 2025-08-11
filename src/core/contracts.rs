//! Runtime contracts for gradual typing

use super::types::{Contract, TypeResolution, Certainty};

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
            Certainty::Unknown => {
                Some(self.create_dynamic_check(resolution))
            }
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
    
    fn create_dynamic_check(&self, _resolution: &TypeResolution) -> Contract {
        Contract {
            check_code: "// Dynamic type - runtime check required".to_string(),
            error_message: "Type cannot be determined statically".to_string(),
        }
    }
    
    fn generate_check_code(&self, resolution: &TypeResolution) -> String {
        match self.mode {
            ContractMode::Warning => {
                format!("// WARNING: Type uncertainty for {:?}", resolution.result)
            }
            ContractMode::Assert => {
                "Если ТипЗнч(Значение) <> ОжидаемыйТип Тогда
                    ВызватьИсключение \"Type mismatch\";
                КонецЕсли;".to_string()
            }
            ContractMode::Report => {
                "// Log type mismatch for analysis".to_string()
            }
        }
    }
}