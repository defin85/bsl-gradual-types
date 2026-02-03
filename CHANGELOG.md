# Changelog

All notable changes to BSL Gradual Type System will be documented in this file.

## [1.1.0] - 2025-11-22 - 🎨 Enhanced UX (Milestone 3.6)

### ✨ Added - Phase 1: Settings & Detail Levels
- **Three detail levels for hover** - Compact (type only), Full (type + methods), Detailed (complete information)
- **VSCode Extension settings** - Configurable detailLevel, maxMethods, maxProperties, showCertainty
- **LSP Server configuration handling** - Thread-safe settings via Arc<RwLock<BslSettings>>
- **Multiline formatting** - Improved readability for methods with 4+ parameters

### ✨ Added - Phase 2: Facets, Generics, Documentation
- **Facets display in hover** - Manager/Object/Reference/Selection/List aspects shown for 1C types
- **Generic types explanation** - Clear display for Массив<T>, Соответствие<K,V>, etc.
- **Documentation links** - Integration with local Syntax Helper and online 1C documentation
- **Auto-detection** - Automatic discovery of syntax_helper_path
- **Windows path handling** - Proper file:// URL generation

### ✨ Added - Phase 3: Enhanced Diagnostic Messages
- **Variable context in errors** - Diagnostics now include variable names (variable_name, param_variable_name)
- **Three detail levels for diagnostics** - Brief/Standard/Detailed message formats
- **Smart hints for error correction** - Contextual suggestions for fixing type errors
- **Graceful degradation** - Fallback to simpler messages when variable context unavailable

### 🏗️ Architecture
- **Unified DetailLevel enum** - Shared between hover and diagnostics (DRY principle)
- **Backward compatibility** - 100% compatible with existing code (zero breaking changes)
- **Builder pattern** - HoverBuilder with chainable methods

### 📊 Testing
- **79 new tests** - 27 (Phase 1) + 16 (Phase 2) + 36 (Phase 3)
- **332 regression tests** - All passing (Backend lib: 106, Shared lib: 226)
- **100% success rate** - 411 total tests passing
- **Code review score** - 4.8/5 stars ⭐⭐⭐⭐⭐

## [1.0.0] - 2025-01-18 - 🏆 ENTERPRISE READY RELEASE

### 🎉 Major Milestones  
- **COMPLETE**: Phases 1-6 fully implemented and tested
- **ENTERPRISE READY**: Production deployment ready
- **ECOSYSTEM COMPLETE**: Full IDE integration and web tooling

### ✨ Added - Phase 6.0 (IDE Integration & Ecosystem)
- **VSCode Extension** - Complete adaptation from bsl_type_safety_analyzer (209 files)
  - Enhanced LSP client with custom request types
  - TypeHintsProvider for inline type information
  - CodeActionsProvider with automatic fixes  
  - PerformanceMonitor for real-time statistics
- **Web-based Type Browser** (`bsl-web-server`)
  - HTTP REST API for browsing types (`/api/types`, `/api/analyze`) 
  - Real-time type search via web interface
  - Live code analysis in browser
  - Performance metrics dashboard

### ✨ Added - Phase 5.0 (Production Readiness)
- **Enhanced LSP Server** - Incremental parsing, enhanced hover, smart completion
- **Performance System** - Profiling with ~189μs parsing, ~125μs type checking  
- **Analysis Caching** - SHA256-based cache with TTL
- **Parallel Analysis** - Rayon integration for batch processing
- **Memory Optimization** - String interning and monitoring
- **Code Actions & Type Hints** - Automatic fixes and inline type display
- **GitHub Actions CI/CD** - Multi-platform testing and automated releases

### ✨ Added - Phase 4.6 (Advanced Type Analysis)
- **Flow-Sensitive Analysis** - State tracking with merge points
- **Full Union Types** - Weighted types with normalization
- **Interprocedural Analysis** - Call graph and topological sorting

### 📊 Performance Benchmarks
- Parsing: ~189μs (excellent)
- Type Checking: ~125μs (production ready)
- Flow Analysis: ~175ns (blazing fast)
- LSP Response: <100ms (responsive)

## [0.5.0] - 2025-01-16 - Phase 4.5 Tree-sitter Migration
### ✨ Added
- Tree-sitter-bsl integration
- Incremental parsing support

## [0.4.0] - 2025-01-15 - Phase 4.0 Extended Analysis
### ✨ Added
- Global functions support
- Type narrowing in conditionals

## [0.3.0] - 2025-01-12 - Phase 3.0 Query Language
### ✨ Added
- 1C query language parser
- Temporary tables and JOIN operations

## [0.2.0] - 2025-01-11 - Phase 2.0 Code Analysis
### ✨ Added
- BSL parser and AST generation
- Type checker with LSP diagnostics

## [0.1.0] - 2025-01-10 - Phase 1.0 MVP
### ✨ Added
- Basic type resolution
- LSP server with hover and completion

---

**🚀 Ready for production deployment in 1C:Enterprise projects!**