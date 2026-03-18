import { CompletionProbe } from './completionProbe';

export const DEFAULT_COMPLETION_PROBE_MAX_ENTRIES = 100;

export class CompletionProbeStore {
    private readonly probes: CompletionProbe[] = [];
    readonly maxEntries: number;

    constructor(maxEntries: number = DEFAULT_COMPLETION_PROBE_MAX_ENTRIES) {
        this.maxEntries = sanitizeMaxEntries(maxEntries);
    }

    get size(): number {
        return this.probes.length;
    }

    add(probe: CompletionProbe): void {
        this.probes.push({ ...probe });
        const overflow = this.probes.length - this.maxEntries;
        if (overflow > 0) {
            this.probes.splice(0, overflow);
        }
    }

    clear(): void {
        this.probes.length = 0;
    }

    snapshot(): CompletionProbe[] {
        return this.probes.map((probe) => ({ ...probe }));
    }
}

function sanitizeMaxEntries(value: number): number {
    if (!Number.isFinite(value)) {
        return DEFAULT_COMPLETION_PROBE_MAX_ENTRIES;
    }

    const normalized = Math.trunc(value);
    if (normalized <= 0) {
        return DEFAULT_COMPLETION_PROBE_MAX_ENTRIES;
    }

    return normalized;
}
