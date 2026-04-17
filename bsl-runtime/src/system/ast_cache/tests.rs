use super::*;
use crate::parsing::bsl::ast::Program;
use crate::parsing::ParseResult;

#[test]
fn ast_cache_hits_misses_and_evictions() {
    let cache = AstCache::new(1);
    let parse = ParseResult::success(Program { statements: vec![] });

    assert!(cache.get([1; 32]).is_none());

    cache.put([1; 32], Arc::new(parse.clone()));
    assert!(cache.get([1; 32]).is_some());

    cache.put([2; 32], Arc::new(parse));
    assert!(cache.get([1; 32]).is_none());

    let stats = cache.stats();
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.evictions, 1);
}

#[test]
fn ast_cache_take_if_unique_preserves_shared_entries() {
    let cache = AstCache::new(2);
    let parse = ParseResult::success(Program { statements: vec![] });

    cache.put([1; 32], Arc::new(parse.clone()));
    assert!(cache.take_if_unique([1; 32]).is_some());
    assert!(cache.get([1; 32]).is_none());

    let shared = Arc::new(parse);
    cache.put([2; 32], Arc::clone(&shared));
    assert!(cache.take_if_unique([2; 32]).is_none());
    assert!(cache.get([2; 32]).is_some());
}
