use bsl_shared::domain::signature_index::ContextRequirements;

pub(crate) fn context_from_directive(
    directive: crate::parsing::bsl::ast::CompilerDirective,
) -> ContextRequirements {
    use crate::parsing::bsl::ast::CompilerDirective as D;
    match directive {
        D::OnServer | D::OnServerNoContext => ContextRequirements::ServerOnly,
        D::OnClient => ContextRequirements::ClientOnly,
        D::OnClientOnServerNoContext => ContextRequirements::Universal,
        D::Unknown => ContextRequirements::default(),
    }
}
