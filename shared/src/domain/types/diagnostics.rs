//! Diagnostic types for type system
//!
//! This module contains types for type-related diagnostics:
//! - `TypeDiagnostic`: Diagnostic message with location
//! - `DiagnosticSeverity`: Error, Warning, Info, Hint
//! - `ParseError`: Syntax error from parser
//! - `ErrorType`: Type of syntax error
//! - `TypeContext`: Symbol table for type resolution
//! - `FunctionSignature`: Placeholder for function signatures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::resolution::TypeResolution;

/// Diagnostic message with location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDiagnostic {
    /// Severity level
    pub severity: DiagnosticSeverity,
    /// Diagnostic message
    pub message: String,
    /// Диапазон ошибки в исходном коде (byte offsets).
    pub span: crate::ir::Span,
}

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Type of syntax error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    /// General parsing error
    ParseError,
    /// Invalid syntax
    InvalidSyntax,
    /// Missing required token (e.g., EndIf)
    MissingToken,
    /// Unexpected token
    UnexpectedToken,
}

/// Related diagnostic location (for unclosed blocks, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedInformation {
    pub message: String,
    pub span: crate::ir::Span,
}

/// Syntax error from parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    /// Error type
    pub error_type: ErrorType,
    /// Error message
    pub message: String,
    /// Error position in source code
    pub span: crate::ir::Span,
    /// Related locations (e.g., opening token for missing end)
    #[serde(default)]
    pub related: Vec<RelatedInformation>,
}

impl ParseError {
    /// Create a missing token error
    pub fn missing_token(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::MissingToken,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// Create an invalid syntax error
    pub fn invalid_syntax(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::InvalidSyntax,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// Create an unexpected token error
    pub fn unexpected_token(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::UnexpectedToken,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// Create a general parsing error
    pub fn new_parse_error(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::ParseError,
            message,
            span,
            related: Vec::new(),
        }
    }
}

/// Type context with symbol table for type resolution
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    /// Symbol table mapping names to resolved types
    pub symbol_table: HashMap<String, TypeResolution>,
}

impl TypeContext {
    /// Create a new empty type context
    pub fn new() -> Self {
        Self::default()
    }
}

/// Placeholder for function signatures (for future use)
#[derive(Debug, Clone)]
pub struct FunctionSignature {}
