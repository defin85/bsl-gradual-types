# BSL Gradual Type System

A modern gradual type system for 1C:Enterprise BSL language that combines static analysis with runtime contracts.

## 🎯 Key Features

- **Gradual Typing** - Smooth transition from dynamic to static typing
- **Evolutionary Architecture** - Start with MVP, extend without breaking changes
- **Type Resolution Pipeline** - Confidence-based type resolution with multiple sources
- **Facet System** - Support for multiple representations of the same type
- **Runtime Contracts** - Dynamic type checking for uncertain cases

## 🚀 Quick Start

```bash
# Build the project
cargo build --release

# Analyze a BSL file
cargo run --bin bsl-analyzer -- --file module.bsl --config /path/to/config

# Start LSP server
cargo run --bin lsp-server
```

## 📊 Architecture Overview

The system is built on a layered architecture that allows incremental development:

### Core Concepts

1. **TypeResolution** - Not a type, but a resolution with confidence level
2. **Certainty Levels** - Known, Inferred(0.0-1.0), Unknown
3. **Facets** - Multiple views of the same type (Manager, Object, Reference, Metadata)
4. **Gradual Info** - Static type + Dynamic contract

### Architecture Layers

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│   (LSP Server, CLI Tools, Extensions)   │
├─────────────────────────────────────────┤
│         Resolution Layer                │
│   (Type Resolver, Context Resolver)     │
├─────────────────────────────────────────┤
│           Core Layer                    │
│   (Types, Facets, Contracts)           │
├─────────────────────────────────────────┤
│         Adapter Layer                   │
│   (Platform Docs, Config Parser)        │
└─────────────────────────────────────────┘
```

## 🔄 Development Roadmap

### ✅ Phase 1: MVP (Completed)
- [x] Basic type resolution with confidence levels
- [x] Full facet system (Manager, Object, Reference, Constructor)
- [x] Core data structures and abstractions
- [x] Platform types loading (hardcoded with TODOs)
- [x] Configuration XML parsing with tabular sections
- [x] LSP server with hover and completion
- [x] Runtime contracts generation

### ✅ Phase 2: Code Analysis (Completed)
- [x] BSL parser using nom combinators
- [x] AST (Abstract Syntax Tree) generation
- [x] Type dependency graph with cycle detection
- [x] Basic type checker with inference
- [x] Type compatibility checking
- [x] Diagnostics in LSP (errors, warnings, info)
- [x] Context-aware type resolution

### 🚀 Phase 3: Advanced Analysis (Next)
- [ ] Query language support
- [ ] Flow-sensitive type analysis
- [ ] Inter-procedural analysis
- [ ] Type narrowing in conditionals
- [ ] Dead code detection

### Phase 4: Platform Integration
- [ ] Platform documentation parser
- [ ] Real platform types from ITS/HTML docs
- [ ] Configuration metadata indexing
- [ ] Cross-module type tracking

### Phase 5: Optimization & ML
- [ ] Incremental analysis
- [ ] Parallel type checking
- [ ] ML-based type predictions
- [ ] Performance profiling tools

## 🏗️ Project Structure

```
bsl-gradual-types/
├── src/
│   ├── core/                # Core type system
│   │   ├── types.rs         # Type definitions & resolution
│   │   ├── resolution.rs    # Type resolver pipeline
│   │   ├── contracts.rs     # Runtime contracts generation
│   │   ├── facets.rs        # Facet system (Manager/Object/Reference)
│   │   ├── context.rs       # Context-aware resolution
│   │   ├── dependency_graph.rs # Type dependency tracking
│   │   ├── type_checker.rs  # Type checking & inference
│   │   └── standard_types.rs # Standard BSL types
│   ├── parser/              # BSL parser
│   │   ├── lexer.rs         # Tokenization
│   │   ├── parser.rs        # Syntax analysis (nom-based)
│   │   ├── ast.rs           # Abstract Syntax Tree
│   │   ├── visitor.rs       # AST visitor pattern
│   │   └── graph_builder.rs # Build dependency graph from AST
│   ├── adapters/            # External data adapters
│   │   ├── config_parser_xml.rs # Configuration.xml parser
│   │   └── platform_docs.rs     # Platform documentation parser
│   └── bin/                 # Binary executables
│       ├── analyzer.rs      # CLI analyzer tool
│       ├── lsp_server.rs    # Language Server Protocol
│       ├── build_index.rs   # Type index builder
│       └── type_check.rs    # Type checker CLI
├── tests/                   # Integration tests
├── docs/                    # Architecture & design docs
└── examples/               # Usage examples
```

## 📚 Documentation

See the `docs/` directory for detailed documentation:

- [Architecture Design](docs/ARCHITECTURE.md)
- [API Reference](docs/API.md)
- [Development Guide](docs/DEVELOPMENT.md)

## 🤝 Contributing

This project is in active development. Contributions are welcome!

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details

## 🔗 Related Projects

- [bsl_type_safety_analyzer](https://github.com/yourusername/bsl_type_safety_analyzer) - Previous iteration
- [1c-syntax](https://github.com/1c-syntax) - BSL language tools

## 📊 Status

**Current Version**: 0.2.0 (Phase 2 Complete)  
**Status**: Active Development  
**Platform Support**: 1C:Enterprise 8.3.20+

### Recent Achievements
- ✅ Complete BSL parser with full syntax support
- ✅ Type inference engine with confidence levels
- ✅ Dependency graph for type flow analysis
- ✅ LSP server with diagnostics and autocomplete
- ✅ Runtime contract generation for gradual typing