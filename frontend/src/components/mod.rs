//! UI Components

pub mod navigation;
pub mod metric_card;
pub mod type_card;
pub mod search_bar;
pub mod type_table;
pub mod graph_view;
pub mod view_switcher;
pub mod dashboard;
pub mod sidebar;
pub mod cards_view;
pub mod table_view;
pub mod pagination;

pub use navigation::Navigation;
pub use metric_card::MetricCard;
pub use type_card::{TypeCard, TypeCardsGrid};
pub use search_bar::{SearchBar, SimpleSearchBar, HeaderSearchBar};
pub use type_table::TypeTable;
pub use graph_view::GraphView;
pub use view_switcher::{ViewSwitcher, ExtendedViewSwitcher, ViewTabs, ViewDropdown, ViewType};
pub use dashboard::Dashboard;
pub use sidebar::Sidebar;
pub use cards_view::CardsView;
pub use table_view::TableView;
pub use pagination::Pagination;
