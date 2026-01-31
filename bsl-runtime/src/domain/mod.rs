//! Backend Domain Layer Extensions
//!
//! Компоненты domain layer, специфичные для backend (работают с AST из parsing)

// TODO: Flow-sensitive анализ переезжает в v2 pipeline (`SemanticProgram.cfg`).
// Устаревшие экспериментальные анализаторы в `bsl-runtime` не развиваем.
pub mod flow_analyzer_simple;

#[allow(deprecated)]
pub use flow_analyzer_simple::{
    FlowAnalysisResult as SimpleFlowAnalysisResult, SimpleFlowAnalyzer,
};
