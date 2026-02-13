//! Backward-compatible alias for the shared IntelliSense v2 facade.
//! Actual orchestration implementation lives in `bsl-runtime`.

pub(crate) type AnalysisV2Runtime = bsl_runtime::application::intellisense_v2::IntellisenseV2Facade;
