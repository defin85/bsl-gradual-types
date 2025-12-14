# FIX RECOMMENDATIONS: Managing Certainty enum Simplification

## RECOMMENDED APPROACH: Gradual Migration with ConfidenceLevel Enum

### Phase 1: Foundation (THIS WEEK - 4 hours)

#### Step 1.1: Create ConfidenceLevel Enum

**File:** `shared/src/domain/types/confidence_level.rs` (NEW)

```rust
//! Confidence levels for type inference
//!
//! Replaces raw f32 values with semantic categories
//! to preserve decision logic while improving code clarity.

use serde::{Deserialize, Serialize};

/// Semantic confidence level for inferred types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// 0.0-0.2: Very low confidence (guessing)
    VeryLow,
    /// 0.2-0.5: Low confidence (uncertain)
    Low,
    /// 0.5-0.8: Medium confidence (reasonable)
    Medium,
    /// 0.8-0.95: High confidence (quite sure)
    High,
    /// 0.95-1.0: Very high confidence (almost certain)
    VeryHigh,
}

impl ConfidenceLevel {
    /// Convert from f32 confidence value (0.0-1.0)
    pub fn from_f32(value: f32) -> Self {
        match value {
            v if v >= 0.95 => ConfidenceLevel::VeryHigh,
            v if v >= 0.80 => ConfidenceLevel::High,
            v if v >= 0.50 => ConfidenceLevel::Medium,
            v if v >= 0.20 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        }
    }

    /// Convert to representative f32 value (midpoint of range)
    pub fn to_f32(&self) -> f32 {
        match self {
            ConfidenceLevel::VeryLow => 0.1,
            ConfidenceLevel::Low => 0.35,
            ConfidenceLevel::Medium => 0.65,
            ConfidenceLevel::High => 0.875,
            ConfidenceLevel::VeryHigh => 0.975,
        }
    }

    /// Convert to percentage for UI display (0-100)
    pub fn to_percent(&self) -> u8 {
        (self.to_f32() * 100.0) as u8
    }

    /// Is this confidence level "acceptable" for strict checks?
    /// (used by CLI tool for warning threshold)
    pub fn is_acceptable_quality(&self) -> bool {
        matches!(
            self,
            ConfidenceLevel::High | ConfidenceLevel::VeryHigh
        )
    }

    /// Should this be considered as "Known" for decision logic?
    /// (used by Generic type inference)
    pub fn should_promote_to_known(&self) -> bool {
        matches!(self, ConfidenceLevel::VeryHigh)
    }

    /// Human-readable label for UI
    pub fn label(&self) -> &'static str {
        match self {
            ConfidenceLevel::VeryLow => "Very Low",
            ConfidenceLevel::Low => "Low",
            ConfidenceLevel::Medium => "Medium",
            ConfidenceLevel::High => "High",
            ConfidenceLevel::VeryHigh => "Very High",
        }
    }

    /// CSS/UI class for badge color
    pub fn ui_class(&self) -> &'static str {
        match self {
            ConfidenceLevel::VeryLow => "danger",     // 🔴
            ConfidenceLevel::Low => "warning",        // 🟡
            ConfidenceLevel::Medium => "info",        // 🔵
            ConfidenceLevel::High => "success",       // 🟢
            ConfidenceLevel::VeryHigh => "success",   // 🟢
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_f32() {
        assert_eq!(ConfidenceLevel::from_f32(0.05), ConfidenceLevel::VeryLow);
        assert_eq!(ConfidenceLevel::from_f32(0.35), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_f32(0.65), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_f32(0.85), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_f32(0.97), ConfidenceLevel::VeryHigh);
    }

    #[test]
    fn test_is_acceptable_quality() {
        assert!(!ConfidenceLevel::VeryLow.is_acceptable_quality());
        assert!(!ConfidenceLevel::Low.is_acceptable_quality());
        assert!(!ConfidenceLevel::Medium.is_acceptable_quality());
        assert!(ConfidenceLevel::High.is_acceptable_quality());
        assert!(ConfidenceLevel::VeryHigh.is_acceptable_quality());
    }

    #[test]
    fn test_should_promote_to_known() {
        assert!(!ConfidenceLevel::VeryLow.should_promote_to_known());
        assert!(!ConfidenceLevel::Medium.should_promote_to_known());
        assert!(ConfidenceLevel::VeryHigh.should_promote_to_known());
    }
}
```

#### Step 1.2: Mark f32 as Deprecated

**File:** `shared/src/domain/types/certainty.rs`

```rust
pub enum Certainty {
    Known,
    /// Type is inferred with given confidence (0.0 - 1.0)
    ///
    /// ⚠️  DEPRECATED in v0.5+
    ///
    /// Migration path: Use ConfidenceLevel enum instead:
    /// ```ignore
    /// // OLD: Certainty::Inferred(0.75)
    /// // NEW: Certainty::Inferred(ConfidenceLevel::High)
    /// ```
    ///
    /// This variant will be removed in v4.0.
    /// See docs/migration/v0.5-confidence-level.md
    #[deprecated(since = "0.5.0", note = "Use ConfidenceLevel enum instead")]
    Inferred(f32),
    Unknown,
}
```

#### Step 1.3: Add New Variant (Future)

```rust
pub enum Certainty {
    Known,
    #[deprecated(since = "0.5.0", note = "Use ConfidenceLevel enum instead")]
    Inferred(f32),  // <-- Old variant kept for compatibility

    // NEW variant coming in v0.6:
    // InferredWithLevel(ConfidenceLevel),

    Unknown,
}
```

#### Step 1.4: Add Migration Guide

**File:** `docs/migration/v0.5-confidence-level.md`

```markdown
# Migration Guide: Certainty enum (v0.5+)

## Overview

In v0.5, we introduce `ConfidenceLevel` enum to replace raw f32 values.
The old `Inferred(f32)` variant is marked as deprecated and will be removed in v4.0.

## Migration Path

### Phase 1 (v0.5): Awareness
- New code should use `ConfidenceLevel` enum
- Old f32 code still works (marked as @deprecated)
- No breaking changes

### Phase 2 (v0.6-v0.7): Migration
- Migrate internal logic to use enum
- CLI tool uses enum for decisions
- Frontend components updated

### Phase 3 (v4.0): Removal
- Remove f32 variant completely
- All code uses enum

## How to Update Your Code

### Before (v0.4)
```rust
certainty: Certainty::Inferred(0.75),

match resolution.certainty {
    Certainty::Inferred(conf) if conf > 0.8 => { ... },
    _ => { ... }
}
```

### After (v0.5+)
```rust
use bsl_shared::domain::types::ConfidenceLevel;

certainty: Certainty::Inferred(ConfidenceLevel::High),

match resolution.certainty {
    Certainty::Inferred(ConfidenceLevel::High | ConfidenceLevel::VeryHigh) => { ... },
    _ => { ... }
}
```

## Automatic Conversion

During Phase 1, you can use the conversion helper:
```rust
let conf_level = ConfidenceLevel::from_f32(0.75);  // Returns High
```
```

---

### Phase 2: New Code Support (Weeks 2-3)

#### Step 2.1: Add New Certainty Variant

Update `certainty.rs`:
```rust
pub enum Certainty {
    Known,
    #[deprecated(since = "0.5.0", note = "Use InferredWithLevel instead")]
    Inferred(f32),  // OLD
    InferredWithLevel(ConfidenceLevel),  // NEW
    Unknown,
}
```

#### Step 2.2: Update Decision Logic (with both paths)

**File:** `shared/src/domain/resolver/context_resolution.rs`

```rust
pub(crate) fn resolve_generic_from_hint(
    &self,
    base_type: &str,
    type_params: &[String],
    certainty: f32,
) -> TypeResolution {
    let concrete_params = /* ... */;

    if concrete_params.is_empty() {
        return self.resolve_expression_sync(base_type);
    }

    let generic_type = GenericType {
        base_type: base_type.to_string(),
        type_params: concrete_params,
    };

    // ✨ NEW: Use enum for decision logic
    let conf_level = ConfidenceLevel::from_f32(certainty);
    let certainty_level = if conf_level.should_promote_to_known() {
        Certainty::Known
    } else {
        Certainty::InferredWithLevel(conf_level)
    };

    // OLD path for compatibility (can be kept or removed)
    #[allow(deprecated)]
    let certainty_level_old = if certainty > 0.9 {
        Certainty::Known
    } else if certainty > 0.5 {
        Certainty::Inferred(certainty)
    } else {
        Certainty::Inferred(0.5)
    };

    TypeResolution {
        result: ResolutionResult::Generic(generic_type),
        certainty: certainty_level,  // Use new
        source: ResolutionSource::Inferred,
        // ...
    }
}
```

#### Step 2.3: Update CLI Tool (with both paths)

**File:** `cli/src/main.rs`

```rust
async fn check_command(
    path: &str,
    _format: &CliOutputFormat,
    verbose: bool,
    _strict: bool,
) -> anyhow::Result<()> {
    let engine = create_analysis_engine().await?;
    let result = engine.analyze_file(path).await?;

    let mut errors = 0;
    let mut warnings = 0;

    for (_, resolution) in &result.type_resolutions {
        match resolution.certainty {
            Certainty::Unknown => errors += 1,

            // NEW path using enum
            Certainty::InferredWithLevel(level) => {
                if !level.is_acceptable_quality() {
                    warnings += 1;
                }
            }

            // OLD path for compatibility
            #[allow(deprecated)]
            Certainty::Inferred(confidence) => {
                if confidence < 0.7 {
                    warnings += 1;
                }
            }

            Certainty::Known => {}
        }
    }

    // ... rest of function
}
```

---

### Phase 3: Full Migration (Weeks 4-5)

#### Step 3.1: Remove Old f32 Path

Remove all:
- `#[allow(deprecated)]`
- Pattern matches on `Certainty::Inferred(f32)`
- References to raw f32 values

#### Step 3.2: Update All Tests

```rust
// Before
assert_eq!(t.certainty, Certainty::Inferred(0.75));

// After
assert_eq!(t.certainty, Certainty::InferredWithLevel(ConfidenceLevel::High));
```

#### Step 3.3: Update Frontend

```rust
// Before (type_card.rs)
let variant = match certainty {
    Certainty::Inferred(c) if c > 0.8 => "success",
    Certainty::Inferred(_) => "warning",
};

// After
let variant = match certainty {
    Certainty::InferredWithLevel(level) => level.ui_class(),
    Certainty::Known => "success",
    Certainty::Unknown => "dark",
};
```

---

### Phase 4: Final Removal (Next Major Release)

Simply remove:
```rust
// DELETE THIS:
#[deprecated(since = "0.5.0")]
Inferred(f32),
```

Everything else already uses enum.

---

## ALTERNATIVE APPROACH (If you want faster results)

### Simplified Enum (No f32 replacement)

```rust
pub enum Certainty {
    Known,
    Inferred,  // No confidence value
    Unknown,
}
```

**RISKS:**
- ❌ Generic type inference breaks (can't decide 0.95 vs 0.75)
- ❌ CLI warnings become meaningless (can't distinguish quality)
- ❌ UI loses percentage display
- ❌ Metrics become useless

**Do NOT use this approach** without solving the 3 critical places.

---

## PRAGMATIC APPROACH: Minimal Changes

If you absolutely must remove f32 quickly:

### Step 1: Keep f32 but hide it

```rust
pub enum Certainty {
    Known,
    Inferred,  // f32 removed from enum
    Unknown,
}

// Add separate field in TypeResolution
pub struct TypeResolution {
    pub certainty: Certainty,
    pub confidence: Option<f32>,  // <-- Keep the value separate
}
```

**PROS:**
- Enum becomes simpler
- f32 value still preserved
- Can still make decisions

**CONS:**
- TypeResolution becomes bigger
- API less clean
- Migration still needed for all code using old pattern

---

## WHICH APPROACH TO USE?

| Approach | Speed | Risk | Quality | Recommendation |
|----------|-------|------|---------|-----------------|
| **Gradual Migration (Recommended)** | 24-32 days spread | LOW | EXCELLENT | ✅ USE THIS |
| **Simplified (no enum)** | 5 days | CRITICAL | POOR | ❌ DO NOT USE |
| **Keep separate f32** | 8 days | MEDIUM | GOOD | ⚠️ FALLBACK ONLY |
| **Direct removal** | 17-25 days | VERY HIGH | BAD | ❌ DO NOT USE |

---

## TESTING STRATEGY

### Phase 1 Testing
- ✅ ConfidenceLevel enum tests (unit)
- ✅ Deprecation warnings compile
- ✅ Existing code still works

### Phase 2 Testing
- ✅ New InferredWithLevel path works
- ✅ Old Inferred(f32) still works
- ✅ Both paths produce same results
- ✅ No regressions

### Phase 3 Testing
- ✅ All old tests updated
- ✅ Decision logic works with enum
- ✅ CLI tool produces correct warnings
- ✅ Frontend displays correctly

### Phase 4 Testing
- ✅ No deprecated warnings
- ✅ All tests pass
- ✅ Clean compilation

---

## COMMUNICATION PLAN

### For Users
> "We're improving the Certainty enum to be more explicit about confidence levels. v0.5 introduces a new ConfidenceLevel enum for clearer code. Old code continues to work (with deprecation warning). Full migration complete by v4.0."

### For Contributors
> "Update your code to use InferredWithLevel(ConfidenceLevel::...) instead of Inferred(f32). See docs/migration/v0.5-confidence-level.md for examples."

### For Changelog
```markdown
## v0.5.0 (Confidence Level Refactoring)

### Added
- New `ConfidenceLevel` enum for type inference confidence
- `Certainty::InferredWithLevel(ConfidenceLevel)` variant

### Deprecated
- `Certainty::Inferred(f32)` - Use `InferredWithLevel` instead
- Raw f32 confidence values in public APIs

### Migration
- See docs/migration/v0.5-confidence-level.md
- Old code continues to work with deprecation warnings
- Migration required by v4.0

### Breaking Changes
- None (v0.5 is fully backward compatible)
```

---

## SUCCESS CRITERIA

- [ ] Phase 1: ConfidenceLevel enum created and tested
- [ ] Phase 1: Deprecation warnings appear when using old code
- [ ] Phase 1: Migration guide published
- [ ] Phase 2: New InferredWithLevel variant works alongside old Inferred
- [ ] Phase 2: Both paths pass tests (no regressions)
- [ ] Phase 2: CLI tool works with both old and new
- [ ] Phase 2: Frontend components support both
- [ ] Phase 3: All critical logic migrated to enum
- [ ] Phase 3: No deprecation warnings in new code
- [ ] Phase 4: f32 variant removed cleanly
- [ ] Phase 4: All tests pass
- [ ] Phase 4: No breaking changes (for users on old API)
