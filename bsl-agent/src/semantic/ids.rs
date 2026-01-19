use blake3::Hasher;

use super::dto::RangeDto;

#[derive(Debug, Clone, Copy)]
pub enum IdPart<'a> {
    Str(&'a str),
    U64(u64),
    U32(u32),
}

impl<'a> IdPart<'a> {
    fn write_into(self, hasher: &mut Hasher) {
        match self {
            IdPart::Str(value) => {
                hasher.update(value.as_bytes());
            }
            IdPart::U64(value) => {
                hasher.update(&value.to_le_bytes());
            }
            IdPart::U32(value) => {
                hasher.update(&value.to_le_bytes());
            }
        }
        hasher.update(&[0]);
    }
}

pub fn stable_id_hex(parts: &[IdPart<'_>]) -> String {
    let mut hasher = Hasher::new();
    for &part in parts {
        part.write_into(&mut hasher);
    }
    hasher.finalize().to_hex().to_string()
}

pub fn document_id(root_id: &str, path: &str) -> String {
    format!("{root_id}:{path}")
}

pub fn diagnostic_id(
    analysis_revision: u64,
    document_id: &str,
    range: &RangeDto,
    code: Option<&str>,
    message: &str,
) -> String {
    stable_id_hex(&[
        IdPart::U64(analysis_revision),
        IdPart::Str(document_id),
        IdPart::U32(range.start.line),
        IdPart::U32(range.start.character),
        IdPart::U32(range.end.line),
        IdPart::U32(range.end.character),
        IdPart::Str(code.unwrap_or("")),
        IdPart::Str(message),
    ])
}

pub fn pack_id(
    analysis_revision: u64,
    goal: &str,
    focus: &str,
    scope: &str,
    include: &str,
    budget_chars: u32,
) -> String {
    stable_id_hex(&[
        IdPart::U64(analysis_revision),
        IdPart::Str(goal),
        IdPart::Str(focus),
        IdPart::Str(scope),
        IdPart::Str(include),
        IdPart::U32(budget_chars),
    ])
}

pub fn pack_item_id(pack_id: &str, kind: &str, primary: &str) -> String {
    stable_id_hex(&[
        IdPart::Str(pack_id),
        IdPart::Str(kind),
        IdPart::Str(primary),
    ])
}
