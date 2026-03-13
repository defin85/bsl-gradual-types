import * as assert from 'assert';
import { resolveRepoBoundConfigValue } from '../../config/configHelper';

suite('Config Helper: repo-bound settings', () => {
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
