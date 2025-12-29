//! Services - business logic modules for TypeSystemService
//!
//! Each service module contains functions that implement specific
//! business operations, receiving &TypeSystemService as a parameter.

pub mod completion_service;
pub mod completion_ranking;
pub mod file_analysis_service;
pub mod hover_service;
pub mod validation_service;
pub mod web_api_service;
