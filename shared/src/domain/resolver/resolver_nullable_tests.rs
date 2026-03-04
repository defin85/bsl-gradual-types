//! Тесты для Nullable Types в TypeResolver (Milestone 2.3 Task 4)

use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::resolver::TypeResolver;
use crate::domain::types::{Certainty, ConcreteType, ResolutionResult};
use std::sync::Arc;

fn create_test_resolver() -> TypeResolver {
    let repo = Arc::new(InMemoryTypeRepository::new());
    TypeResolver::new(repo)
}

#[cfg(test)]
#[path = "resolver_nullable_tests/tests.rs"]
mod tests;
