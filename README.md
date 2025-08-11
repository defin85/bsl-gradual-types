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

### Phase 1: MVP (Current)
- [x] Basic type resolution
- [x] Simple facet system
- [x] Core data structures
- [ ] Platform types loading
- [ ] Configuration parsing
- [ ] Basic LSP integration

### Phase 2: Facets & Context
- [ ] Full facet support
- [ ] Context-aware resolution
- [ ] Method transitions between facets

### Phase 3: Type Inference
- [ ] Constraint collection
- [ ] Type inference engine
- [ ] Chain resolution

### Phase 4: Runtime Contracts
- [ ] Contract generation
- [ ] Runtime check injection
- [ ] Configurable strictness

### Phase 5: Advanced Features
- [ ] Flow analysis
- [ ] ML predictions
- [ ] Performance optimizations

## 🏗️ Project Structure

```
bsl-gradual-types/
├── src/
│   ├── core/           # Core type system
│   │   ├── types.rs    # Type definitions
│   │   ├── resolution.rs # Type resolver
│   │   ├── contracts.rs  # Runtime contracts
│   │   ├── facets.rs     # Facet system
│   │   └── context.rs    # Context handling
│   ├── adapters/       # External data adapters
│   └── bin/           # Binary executables
├── tests/             # Integration tests
├── docs/             # Documentation
└── examples/         # Usage examples
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

**Current Version**: 0.1.0 (MVP)  
**Status**: Active Development  
**Platform Support**: 1C:Enterprise 8.3.20+