import * as assert from 'assert';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { findConfigurations } from '../../utils/configurationFinder';

function writeConfigurationXml(configRoot: string, name: string): void {
    fs.mkdirSync(configRoot, { recursive: true });
    fs.writeFileSync(
        path.join(configRoot, 'Configuration.xml'),
        `<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>${name}</Name>
    </Properties>
  </Configuration>
</MetaDataObject>
`,
        'utf-8'
    );
}

suite('Configuration Finder Test Suite', () => {
    test('findConfigurations detects configuration when root path is config root', async () => {
        const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'bsl-config-root-'));
        try {
            writeConfigurationXml(tempRoot, 'RootConfig');

            const configurations = await findConfigurations(tempRoot);

            assert.strictEqual(configurations.length, 1);
            assert.strictEqual(configurations[0].path, tempRoot);
            assert.strictEqual(configurations[0].name, path.basename(tempRoot));
            assert.strictEqual(configurations[0].isExtension, false);
        } finally {
            fs.rmSync(tempRoot, { recursive: true, force: true });
        }
    });

    test('findConfigurations still detects configuration in child directory', async () => {
        const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'bsl-config-child-'));
        try {
            const childConfig = path.join(tempRoot, 'conf');
            writeConfigurationXml(childConfig, 'ChildConfig');

            const configurations = await findConfigurations(tempRoot);

            assert.strictEqual(configurations.length, 1);
            assert.strictEqual(configurations[0].path, childConfig);
            assert.strictEqual(configurations[0].name, 'conf');
            assert.strictEqual(configurations[0].isExtension, false);
        } finally {
            fs.rmSync(tempRoot, { recursive: true, force: true });
        }
    });
});
