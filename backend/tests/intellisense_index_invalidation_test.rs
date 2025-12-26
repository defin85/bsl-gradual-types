use std::sync::{Arc, Barrier};

use bsl_backend::system::intellisense_index::{
    IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, SymbolKind,
};

fn symbol_item(name: &str) -> IndexItem {
    IndexItem::new(
        name,
        IndexItemKind::Symbol(SymbolKind::Variable),
        IndexKind::Symbol,
    )
}

#[test]
fn invalidate_file_removes_only_related_entries() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    let uri_a = "file:///a.bsl";
    let uri_b = "file:///b.bsl";
    let key_a = "module-a";
    let key_b = "module-b";

    store.replace_symbols_for_uri(uri_a, vec![symbol_item("A")]);
    store.replace_symbols_for_uri(uri_b, vec![symbol_item("B")]);
    store.replace_modules_for_key(key_a, vec![symbol_item("Amod")]);
    store.replace_modules_for_key(key_b, vec![symbol_item("Bmod")]);

    store.invalidate_file(uri_a, Some(key_a));

    let snapshot = store.snapshot();
    assert!(snapshot.symbol_index.get(uri_a).is_none());
    assert!(snapshot.module_index.get(key_a).is_none());
    assert!(snapshot.symbol_index.get(uri_b).is_some());
    assert!(snapshot.module_index.get(key_b).is_some());
}

#[test]
fn snapshot_stays_consistent_under_parallel_updates() {
    let store = Arc::new(IntellisenseIndexStore::new("cfg", "platform"));
    let uri_a = "file:///a.bsl";
    let uri_b = "file:///b.bsl";
    let key_a = "module-a";
    let key_b = "module-b";

    store.replace_symbols_for_uri(uri_b, vec![symbol_item("B")]);
    store.replace_modules_for_key(key_b, vec![symbol_item("Bmod")]);

    let barrier = Arc::new(Barrier::new(3));
    let store_writer = Arc::clone(&store);
    let barrier_writer = Arc::clone(&barrier);
    let writer = std::thread::spawn(move || {
        barrier_writer.wait();
        for _ in 0..200 {
            store_writer.replace_symbols_for_uri(uri_a, vec![symbol_item("A")]);
            store_writer.replace_modules_for_key(key_a, vec![symbol_item("Amod")]);
        }
    });

    let store_invalidator = Arc::clone(&store);
    let barrier_invalidator = Arc::clone(&barrier);
    let invalidator = std::thread::spawn(move || {
        barrier_invalidator.wait();
        for _ in 0..200 {
            store_invalidator.invalidate_file(uri_a, Some(key_a));
        }
    });

    barrier.wait();
    writer.join().expect("writer thread");
    invalidator.join().expect("invalidator thread");

    store.invalidate_file(uri_a, Some(key_a));

    let snapshot = store.snapshot();
    assert!(snapshot.symbol_index.get(uri_a).is_none());
    assert!(snapshot.module_index.get(key_a).is_none());
    assert!(snapshot.symbol_index.get(uri_b).is_some());
    assert!(snapshot.module_index.get(key_b).is_some());
}
