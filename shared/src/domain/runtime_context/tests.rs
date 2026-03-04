use super::*;

#[test]
fn test_new_context_has_unknown_directive() {
    let ctx = RuntimeExecutionContext::new();
    assert_eq!(ctx.current_directive, CompilerDirective::Unknown);
    assert!(ctx.in_function.is_none());
}

#[test]
fn test_server_context_can_call_all_methods() {
    let mut ctx = RuntimeExecutionContext::new();
    ctx.current_directive = CompilerDirective::OnServer;

    assert!(ctx.can_call_method(&ContextRequirements::ServerOnly));
    assert!(ctx.can_call_method(&ContextRequirements::ClientOnly));
    assert!(ctx.can_call_method(&ContextRequirements::Universal));
}

#[test]
fn test_client_context_cannot_call_server_only() {
    let mut ctx = RuntimeExecutionContext::new();
    ctx.current_directive = CompilerDirective::OnClient;

    assert!(!ctx.can_call_method(&ContextRequirements::ServerOnly));
    assert!(ctx.can_call_method(&ContextRequirements::ClientOnly));
    assert!(ctx.can_call_method(&ContextRequirements::Universal));
}

#[test]
fn test_universal_context_only_allows_universal() {
    let mut ctx = RuntimeExecutionContext::new();
    ctx.current_directive = CompilerDirective::OnClientOnServerNoContext;

    assert!(!ctx.can_call_method(&ContextRequirements::ServerOnly));
    assert!(!ctx.can_call_method(&ContextRequirements::ClientOnly));
    assert!(ctx.can_call_method(&ContextRequirements::Universal));
}

#[test]
fn test_unknown_context_allows_all() {
    let ctx = RuntimeExecutionContext::new();

    assert!(ctx.can_call_method(&ContextRequirements::ServerOnly));
    assert!(ctx.can_call_method(&ContextRequirements::ClientOnly));
    assert!(ctx.can_call_method(&ContextRequirements::Universal));
}
