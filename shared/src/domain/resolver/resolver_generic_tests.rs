//! Тесты для Generic Types в TypeResolver (Milestone 2.3 Task 3)

use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::resolver::{GenericStrategy, TypeResolver};
use crate::domain::types::{Certainty, GenericType, ResolutionResult};
use std::sync::Arc;

fn create_test_resolver() -> TypeResolver {
    let repo = Arc::new(InMemoryTypeRepository::new());
    TypeResolver::new(repo)
}

#[cfg(test)]
#[path = "resolver_generic_tests/tests.rs"]
mod tests;
