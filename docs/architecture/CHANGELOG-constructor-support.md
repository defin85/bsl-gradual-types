# Changelog: Constructor Support Implementation

## Summary

Реализован IR узел `NewExpression` для поддержки конструкторов в BSL Type System.

**Date:** 2025-11-05
**Version:** 0.4.2
**Status:** ✅ Completed (Step 1 of Constructor Support Plan)

## Changes

### Added

#### 1. IR Node: `SemanticNodeKind::NewExpression`

**File:** `shared/src/ir/mod.rs` (lines 164-218)

Новый вариант enum для представления конструкторов:

```rust
SemanticNodeKind::NewExpression {
    type_name: String,
    arg_types: Vec<String>,
    is_dynamic: bool,
    result_type: String,
    generic_params: Option<Vec<String>>,
}
```

**Features:**
- ✅ Поддержка простых конструкторов: `Новый Массив`
- ✅ Поддержка конструкторов с параметрами: `Новый Массив(10)`
- ✅ Поддержка динамических конструкторов: `Новый("Тип")`
- ✅ Поддержка Generic параметров: `Массив<Число>`

#### 2. DTO Conversion

**File:** `shared/src/ir/mod.rs` (lines 1006-1036)

Добавлена обработка `NewExpression` в методе `extract_node_info()`:

```rust
SemanticNodeKind::NewExpression { ... } => {
    attributes.insert("type_name", ...);
    attributes.insert("arg_count", ...);
    attributes.insert("is_dynamic", ...);
    // ...
    ("NewExpression", Some(display_name), attributes)
}
```

**Display Format:**
- `Новый Массив` → "Новый Массив"
- `Новый Массив(10)` → "Новый Массив(1 args)"
- `Новый("Тип")` → "Новый(\"Тип\")"

#### 3. Unit Tests

**File:** `shared/src/ir/mod.rs` (lines 1605-1758)

Добавлено 5 новых unit тестов:

1. ✅ `test_new_expression_simple` - простой конструктор без параметров
2. ✅ `test_new_expression_with_args` - конструктор с параметрами
3. ✅ `test_new_expression_dynamic` - динамический конструктор
4. ✅ `test_new_expression_with_generics` - Generic конструктор
5. ✅ `test_new_expression_to_dto` - конверсия в DTO

**Test Results:** All tests passed (158 total, +5 new)

#### 4. Documentation

**Created Files:**
- `docs/architecture/constructor-support.md` - детальная документация
- `docs/architecture/CHANGELOG-constructor-support.md` - этот файл

**Updated Files:**
- `docs/README.md` - добавлена ссылка на architecture раздел

## Test Results

### Before
```
test result: ok. 153 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### After
```
test result: ok. 158 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**New Tests (5):**
- `ir::tests::test_new_expression_simple`
- `ir::tests::test_new_expression_with_args`
- `ir::tests::test_new_expression_dynamic`
- `ir::tests::test_new_expression_with_generics`
- `ir::tests::test_new_expression_to_dto`

## Compilation Status

✅ **Full workspace compiles successfully**

```
Compiling bsl-shared v0.4.2
Compiling bsl-type-visualization v0.1.0
Compiling bsl-backend v0.4.2
Compiling bsl-frontend v0.4.2
Compiling bsl-cli v0.4.2
Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.85s
```

## Code Coverage

### Modified Files

1. `shared/src/ir/mod.rs`
   - Added: `NewExpression` variant (54 lines)
   - Added: DTO conversion logic (30 lines)
   - Added: 5 unit tests (153 lines)
   - Total additions: ~237 lines

### Impact Analysis

**No breaking changes:**
- Existing code continues to work
- New variant added to enum (requires exhaustive match updates)
- All existing tests pass

## Next Steps

### Step 2: AstToIrConverter (Backend)

**File:** `backend/src/converter/ast_to_ir.rs`

**Tasks:**
- [ ] Implement parsing of `Новый` keyword
- [ ] Extract type name and arguments
- [ ] Detect dynamic constructors `Новый("string")`
- [ ] Create `SemanticNodeKind::NewExpression` nodes

**Complexity:** Medium
**Estimated Time:** 2-3 hours

### Step 3: AnalysisEngine (Shared)

**File:** `shared/src/engine.rs`

**Tasks:**
- [ ] Add constructor validation
- [ ] Implement Generic inference for collections
- [ ] Update `result_type` based on inference
- [ ] Add constructor-specific diagnostics

**Complexity:** Medium-High
**Estimated Time:** 3-4 hours

### Step 4: TypeResolver Integration (Shared)

**File:** `shared/src/domain/resolver.rs`

**Tasks:**
- [ ] Add `resolve_constructor()` method
- [ ] Handle Generic parameter resolution
- [ ] Support facet resolution for platform types
- [ ] Integration tests

**Complexity:** High
**Estimated Time:** 4-5 hours

## Related Documentation

- [Constructor Support Architecture](constructor-support.md)
- [Type System Architecture](type_system_architecture.md)
- [Components Detailed](components-detailed.md)

## Version Control

```bash
# Files changed
modified:   shared/src/ir/mod.rs (+237 lines)
created:    docs/architecture/constructor-support.md
created:    docs/architecture/CHANGELOG-constructor-support.md
modified:   docs/README.md (+8 lines)

# Commit suggestion
git add shared/src/ir/mod.rs
git add docs/architecture/constructor-support.md
git add docs/architecture/CHANGELOG-constructor-support.md
git add docs/README.md

git commit -m "feat(ir): add NewExpression node for constructor support

- Add SemanticNodeKind::NewExpression variant with full Generic support
- Implement DTO conversion for NewExpression nodes
- Add 5 comprehensive unit tests (all passing)
- Create detailed architecture documentation
- Update docs navigation

Part of Constructor Support implementation (Step 1/4)
Related to Milestone: Generic Collections Type Inference"
```

---

**Last Updated:** 2025-11-05
**Version:** 0.4.2
**Status:** ✅ Ready for Step 2 (Backend AST Conversion)
