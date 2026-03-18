import * as assert from 'assert';
import { buildCompletionProbe } from '../../providers/completionProbe';
import {
    CompletionProbeStore,
    DEFAULT_COMPLETION_PROBE_MAX_ENTRIES,
} from '../../providers/completionProbeStore';

function makeProbe(id: string, version: number) {
    return buildCompletionProbe({
        probe_id: id,
        uri: `file:///tmp/${id}.bsl`,
        document_version: version,
        trigger_mode: 'invoked',
        request_started_at_ms: 1_700_000_000_000 + version,
        request_completed_at_ms: 1_700_000_000_010 + version,
        client_terminal_state: 'ok_non_empty',
        time_since_last_local_edit_ms: version,
        time_since_last_did_change_sent_ms: 'unknown',
        is_after_dot: false,
        identifier_tail_length: 0,
    });
}

suite('Completion Probe Store Test Suite', () => {
    test('store keeps probes in insertion order until capacity is reached', () => {
        const store = new CompletionProbeStore(3);

        store.add(makeProbe('probe-1', 1));
        store.add(makeProbe('probe-2', 2));

        const snapshot = store.snapshot();
        assert.deepStrictEqual(snapshot.map((probe) => probe.probe_id), [
            'probe-1',
            'probe-2',
        ]);
        assert.strictEqual(store.size, 2);
    });

    test('store evicts the oldest probe first when capacity is exceeded', () => {
        const store = new CompletionProbeStore(2);

        store.add(makeProbe('probe-1', 1));
        store.add(makeProbe('probe-2', 2));
        store.add(makeProbe('probe-3', 3));

        const snapshot = store.snapshot();
        assert.deepStrictEqual(snapshot.map((probe) => probe.probe_id), [
            'probe-2',
            'probe-3',
        ]);
    });

    test('store snapshot returns detached copies and capacity falls back to default when invalid', () => {
        const store = new CompletionProbeStore(0);
        const probe = makeProbe('probe-1', 1);
        store.add(probe);

        const snapshot = store.snapshot();
        snapshot[0].probe_id = 'mutated';

        const secondSnapshot = store.snapshot();
        assert.strictEqual(store.maxEntries, DEFAULT_COMPLETION_PROBE_MAX_ENTRIES);
        assert.strictEqual(secondSnapshot[0].probe_id, 'probe-1');
    });
});
