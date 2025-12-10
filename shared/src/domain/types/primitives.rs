//! Primitive and special types for BSL type system
//!
//! This module contains basic type definitions:
//! - `PrimitiveType`: String, Number, Boolean, Date
//! - `SpecialType`: Undefined, Null, Type

use serde::{Deserialize, Serialize};
use std::fmt;

/// Primitive types in BSL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Date,
}

impl PrimitiveType {
    /// Returns the Russian display name for the primitive type
    pub fn display_name(&self) -> &'static str {
        match self {
            PrimitiveType::String => "Строка",
            PrimitiveType::Number => "Число",
            PrimitiveType::Boolean => "Булево",
            PrimitiveType::Date => "Дата",
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Special types in BSL (Undefined, Null, Type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialType {
    Undefined,
    Null,
    Type,
}

impl SpecialType {
    /// Returns the Russian display name for the special type
    pub fn display_name(&self) -> &'static str {
        match self {
            SpecialType::Undefined => "Неопределено",
            SpecialType::Null => "Null",
            SpecialType::Type => "Тип",
        }
    }
}

impl fmt::Display for SpecialType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
