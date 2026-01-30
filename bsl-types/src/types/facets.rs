//! Facet system for 1C types
//!
//! Based on Balyuk & Popova (2021) scientific paper.
//! One 1C type = multiple representations: Manager, Object, Reference, Selection, List
//!
//! This module contains:
//! - `FacetKind`: All possible facets for configuration types

use serde::{Deserialize, Serialize};

/// Facet kind for configuration types
///
/// Based on the scientific paper by Balyuk & Popova (2021):
/// - Manager: Creation, search (CatalogManager)
/// - Object: Mutable object (CatalogObject)
/// - Reference: Reference to element (CatalogRef)
/// - Selection: Element traversal (CatalogSelection)
/// - List: List management in form (CatalogList)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetKind {
    /// Manager facet - for creation and search
    Manager,
    /// Object facet - mutable object
    Object,
    /// Reference facet - reference to element
    Reference,
    /// Metadata facet - metadata access
    Metadata,
    /// Constructor facet - for object creation
    Constructor,
    /// Collection facet - collection type
    Collection,
    /// Singleton facet - single object
    Singleton,
    /// Selection facet - element traversal (from Balyuk & Popova paper)
    Selection,
    /// List facet - list management in form (from Balyuk & Popova paper)
    List,
}

impl FacetKind {
    /// Returns the Russian display name for the facet
    pub fn display_name(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Metadata => "Метаданные",
            FacetKind::Constructor => "Конструктор",
            FacetKind::Collection => "Коллекция",
            FacetKind::Singleton => "Одиночный",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
        }
    }

    /// Returns the platform suffix for the facet
    pub fn platform_suffix(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
            _ => "",
        }
    }

    /// Whether the facet shows properties in hover
    ///
    /// Object, Reference, Collection - show properties (they represent object data)
    /// Manager, Selection, List - do NOT show properties (they are access/navigation facets)
    pub fn shows_properties(&self) -> bool {
        matches!(
            self,
            FacetKind::Object | FacetKind::Reference | FacetKind::Collection
        )
    }

    /// Whether properties are read-only for this facet
    ///
    /// Reference - reference to element, properties are read-only
    /// Object - mutable object, properties are writable
    pub fn properties_are_readonly(&self) -> bool {
        matches!(self, FacetKind::Reference)
    }
}
