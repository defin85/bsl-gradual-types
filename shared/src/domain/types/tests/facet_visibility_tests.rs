//! Tests for facet visibility logic

use crate::domain::types::*;

#[test]
fn test_manager_hides_properties() {
    assert!(!FacetKind::Manager.shows_properties());
}

#[test]
fn test_object_shows_properties() {
    assert!(FacetKind::Object.shows_properties());
    assert!(!FacetKind::Object.properties_are_readonly());
}

#[test]
fn test_reference_shows_readonly_properties() {
    assert!(FacetKind::Reference.shows_properties());
    assert!(FacetKind::Reference.properties_are_readonly());
}

#[test]
fn test_selection_hides_properties() {
    assert!(!FacetKind::Selection.shows_properties());
}

#[test]
fn test_list_hides_properties() {
    assert!(!FacetKind::List.shows_properties());
}

#[test]
fn test_collection_shows_properties() {
    assert!(FacetKind::Collection.shows_properties());
}

#[test]
fn test_metadata_hides_properties() {
    // Metadata - describes structure, not object data
    assert!(!FacetKind::Metadata.shows_properties());
}

#[test]
fn test_constructor_hides_properties() {
    // Constructor - for creating objects, not for property access
    assert!(!FacetKind::Constructor.shows_properties());
}

#[test]
fn test_singleton_hides_properties() {
    // Singleton - single object, usually a manager
    assert!(!FacetKind::Singleton.shows_properties());
}
