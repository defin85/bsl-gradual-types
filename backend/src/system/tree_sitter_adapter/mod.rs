pub use bsl_syntax::tree_sitter_adapter::{
    TreeSitterAdapter, collect_syntax_errors, collect_syntax_errors_cached,
};

pub mod directives {
    pub use bsl_syntax::tree_sitter_adapter::directives::*;
}

pub mod span {
    pub use bsl_syntax::tree_sitter_adapter::span::*;
}

pub mod utils {
    pub use bsl_syntax::tree_sitter_adapter::utils::*;
}

