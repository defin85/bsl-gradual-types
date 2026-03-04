//! Тесты для Intersection Types в TypeResolver (Milestone 2.3 Task 2)

use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::resolver::TypeResolver;
use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, TypeResolution};
use std::sync::Arc;

fn create_test_resolver() -> TypeResolver {
    let repo = Arc::new(InMemoryTypeRepository::new());
    TypeResolver::new(repo)
}

#[cfg(test)]
#[path = "resolver_intersection_tests/tests.rs"]
mod tests;
