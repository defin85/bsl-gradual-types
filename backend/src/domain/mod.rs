//! Backend Domain Layer Extensions
//!
//! Компоненты domain layer, специфичные для backend (работают с AST из parsing)

// TODO: Полный FlowAnalyzer будет реализован после доработки AST
// pub mod flow_analyzer;
pub mod flow_analyzer_simple;

// pub use flow_analyzer::{FlowAnalyzer, AnalysisResult as FlowAnalysisResult};
pub use flow_analyzer_simple::{
    FlowAnalysisResult as SimpleFlowAnalysisResult, SimpleFlowAnalyzer,
};
