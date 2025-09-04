# 📋 Simplified Architecture Migration Summary

## 🎯 **КРАТКАЯ СВОДКА**

**Цель**: 100% соответствие `docs/simplified_architecture.md`  
**Текущий статус**: 87/100  
**Время выполнения**: 8 дней  
**Основная проблема**: Dual architecture (новая + старая сосуществуют)

---

## 🚨 **КРИТИЧЕСКИЕ ДЕЙСТВИЯ**

### **1. Удалить Legacy Architecture (Priority 1)**
```bash
# Удалить ~3200 LOC legacy кода:
rm src/system/coordination.rs           # CentralTypeSystem (25-30 компонентов)
rm src/system/analysis_cache.rs         # AdvancedAnalysisCache  
rm src/system/performance.rs            # PerformanceProfiler
rm src/system/memory_optimization.rs    # Memory optimization
rm src/system/parallel_analysis.rs      # ParallelAnalysisEngine  
rm src/system/computation.rs            # Helper computations
```

### **2. Переместить TypeSystemService (Priority 1)**
```bash
# Architectural fix:
# src/system/system_coordinator.rs → src/application/type_system_service.rs
# SystemCoordinator должен создавать Application Layer, не содержать TypeSystemService
```

### **3. Ключевые переименования классов**
| Текущее | Требуемое | Файл |
|---------|-----------|------|
| `BslLanguageServer` | `LSPServer` | `src/bin/lsp_server.rs` |
| `BasicTypeResolver` | `TypeResolver` | `src/domain/resolution_service.rs` |
| `PlatformTypesRepository` | `PlatformTypes` | `src/data/platform_types.rs` |
| `ConfigurationGuidedParser` | `ConfigData` | `src/data/config_data.rs` |

---

## 📊 **LAYER COMPLIANCE STATUS**

| Layer | Components | Status | Action |
|-------|------------|--------|---------|
| **System** | SystemCoordinator, AnalysisCache, ParserCoordinator, BasicObservability | ✅ 100% | None |
| **Application** | TypeSystemService | ❌ Wrong layer | **Move from System** |
| **Presentation** | LSPServer, WebInterface, CLITool | ⚠️ 33% | **Rename + Create** |
| **Domain** | TypeResolver, TypeRepository | ⚠️ 50% | **Rename classes** |
| **Data** | PlatformTypes, ConfigData | ⚠️ 0% | **Rename classes** |

---

## 🗓️ **8-DAY TIMELINE**

| Day | Phase | Key Tasks | Deliverable |
|-----|-------|-----------|-------------|
| 1-2 | Architecture Cleanup | Delete legacy, move TypeSystemService | Clean architecture |
| 3-4 | Layer Restructuring | Create Application Layer, Web/CLI | Proper layers |  
| 5 | Class Renaming | Rename all components per docs | Consistent naming |
| 6 | Architecture Validation | Test flows, integration tests | Validated system |
| 7-8 | Documentation | Update docs, migration guide | Complete docs |

---

## 🎯 **SUCCESS METRICS**

- [ ] **Architecture**: 100% compliance with simplified_architecture.md
- [ ] **Components**: 6 components (SystemCoordinator architecture)  
- [ ] **Legacy**: CentralTypeSystem completely removed
- [ ] **Tests**: 101/101 tests passing with new architecture
- [ ] **Performance**: No regression vs current system

---

## 📋 **QUICK START COMMANDS**

```bash
# 1. Create feature branch
git checkout -b feature/simplified-architecture-migration

# 2. Start with critical cleanup
rm src/system/coordination.rs
rm src/system/analysis_cache.rs  
rm src/system/performance.rs
rm src/system/memory_optimization.rs
rm src/system/parallel_analysis.rs
rm src/system/computation.rs

# 3. Create Application Layer
mkdir -p src/application
touch src/application/mod.rs
touch src/application/type_system_service.rs
touch src/application/application_layer.rs

# 4. Run tests to see what breaks
cargo test

# 5. Fix imports and architecture step by step
```

---

## 🔄 **RENAMING QUICK REFERENCE**

### **Files to rename:**
```bash
# Data Layer:
mv src/data/loaders/platform_types_repository.rs src/data/platform_types.rs
mv src/data/loaders/config_parser_guided_discovery.rs src/data/config_data.rs
```

### **Classes to rename:**
```rust
// Domain Layer:
BasicTypeResolver → TypeResolver

// Data Layer:  
PlatformTypesRepository → PlatformTypes
ConfigurationGuidedParser → ConfigData

// Presentation Layer:
BslLanguageServer → LSPServer
```

### **Architecture fixes:**
```rust
// Move TypeSystemService from System → Application Layer
// SystemCoordinator creates ApplicationLayer
// All Presentation layer uses ApplicationLayer, not SystemCoordinator directly
```

---

## 🚀 **EXPECTED RESULT**

**Before**: Dual architecture, 25-30 components, unclear layer boundaries  
**After**: Single clean architecture, 6 components, perfect docs compliance

**Key improvement**: Simplified, maintainable, documented architecture matching vision! 🎯

---

See detailed plans:
- 📋 [SIMPLIFIED_ARCHITECTURE_ROADMAP.md](./SIMPLIFIED_ARCHITECTURE_ROADMAP.md)
- 🔄 [CLASS_RENAMING_PLAN.md](./CLASS_RENAMING_PLAN.md)
