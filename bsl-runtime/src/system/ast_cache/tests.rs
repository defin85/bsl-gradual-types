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
