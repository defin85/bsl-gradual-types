use std::collections::BTreeSet;

/// Домен для межпроцедурного вывода возвращаемых типов (summary-based).
///
/// Контракт:
/// - `known` хранит известные варианты типов (union) и монотонно растёт (через объединение множеств).
/// - `has_dynamic` помечает неопределённость/динамику и **не** должна затирать `known`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReturnDomain {
    pub known: BTreeSet<String>,
    pub has_dynamic: bool,
}

impl ReturnDomain {
    pub fn add_known(&mut self, t: String) {
        if !t.trim().is_empty() {
            self.known.insert(t);
        }
    }

    pub fn join(&mut self, other: &ReturnDomain) {
        self.known.extend(other.known.iter().cloned());
        self.has_dynamic |= other.has_dynamic;
    }

    pub fn to_union_string(&self) -> Option<String> {
        if self.known.is_empty() {
            return None;
        }
        Some(self.known.iter().cloned().collect::<Vec<_>>().join(" | "))
    }
}
