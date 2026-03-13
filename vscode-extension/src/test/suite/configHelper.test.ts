import * as assert from 'assert';
import {
    BSL_EXTENSION_ID,
    buildBslExtensionSettingsQuery,
    resolveRepoBoundConfig,
    resolveRepoBoundConfigValue,
} from '../../config/configHelper';

suite('Config Helper: repo-bound settings', () => {
    test('builds settings query for the whole extension', () => {
        const query = buildBslExtensionSettingsQuery();
        assert.strictEqual(query, `@ext:${BSL_EXTENSION_ID}`);
    });

    test('builds settings query with additional search terms', () => {
        const query = buildBslExtensionSettingsQuery('configurationPath', 'platformDocsArchive');
        assert.strictEqual(
            query,
            `@ext:${BSL_EXTENSION_ID} configurationPath platformDocsArchive`
        );
    });

    test('prefers workspace value over global when workspace is open', () => {
        const value = resolveRepoBoundConfigValue(
            {
                defaultValue: '',
                globalValue: '/global/conf',
                workspaceValue: '/workspace/conf',
            },
            '',
            true
        );

        assert.strictEqual(value, '/workspace/conf');
    });

    test('ignores global fallback when workspace is open', () => {
        const value = resolveRepoBoundConfigValue(
            {
                defaultValue: '',
                globalValue: '/global/conf',
            },
            '',
            true
        );

        assert.strictEqual(value, '');
    });

    test('exposes ignored global value when workspace is open', () => {
        const resolution = resolveRepoBoundConfig(
            {
                defaultValue: '',
                globalValue: '/global/docs',
            },
            '',
            true
        );

        assert.strictEqual(resolution.value, '');
        assert.strictEqual(resolution.ignoredGlobalValue, '/global/docs');
    });

    test('prefers workspace folder over workspace value', () => {
        const value = resolveRepoBoundConfigValue(
            {
                defaultValue: '8.3.25',
                workspaceValue: '8.3.26',
                workspaceFolderValue: '8.3.27',
                globalValue: '8.3.99',
            },
            '8.3.25',
            true
        );

        assert.strictEqual(value, '8.3.27');
    });

    test('keeps global fallback when no workspace is open', () => {
        const value = resolveRepoBoundConfigValue(
            {
                defaultValue: '',
                globalValue: '/global/docs',
            },
            '',
            false
        );

        assert.strictEqual(value, '/global/docs');
    });
});
