const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const os = require('os');
const path = require('path');

const { atomicReplaceFile } = require('./copy-binaries');

function withTempDir(run) {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'copy-binaries-'));
    try {
        run(tempDir);
    } finally {
        fs.rmSync(tempDir, { recursive: true, force: true });
    }
}

test('atomicReplaceFile copies to temp path and then renames into destination', () => {
    withTempDir(tempDir => {
        const sourcePath = path.join(tempDir, 'source.bin');
        const targetPath = path.join(tempDir, 'target.bin');
        fs.writeFileSync(sourcePath, 'new-binary');
        fs.writeFileSync(targetPath, 'old-binary');

        const operations = [];
        const fsSpy = {
            copyFileSync(src, dest) {
                operations.push(['copy', src, dest]);
                return fs.copyFileSync(src, dest);
            },
            renameSync(src, dest) {
                operations.push(['rename', src, dest]);
                return fs.renameSync(src, dest);
            },
            unlinkSync(filePath) {
                operations.push(['unlink', filePath]);
                return fs.unlinkSync(filePath);
            }
        };

        atomicReplaceFile(sourcePath, targetPath, fsSpy);

        assert.equal(fs.readFileSync(targetPath, 'utf8'), 'new-binary');
        assert.equal(operations.length, 2);
        assert.deepEqual(
            operations.map(([kind]) => kind),
            ['copy', 'rename']
        );
        assert.notEqual(
            operations[0][2],
            targetPath,
            'replacement must not write directly into the executing destination inode'
        );
        assert.equal(operations[1][2], targetPath);
    });
});

test('atomicReplaceFile removes temp file when rename fails', () => {
    withTempDir(tempDir => {
        const sourcePath = path.join(tempDir, 'source.bin');
        const targetPath = path.join(tempDir, 'target.bin');
        fs.writeFileSync(sourcePath, 'new-binary');

        let tempPath = null;
        const fsSpy = {
            copyFileSync(src, dest) {
                tempPath = dest;
                return fs.copyFileSync(src, dest);
            },
            renameSync() {
                throw new Error('rename failed');
            },
            unlinkSync(filePath) {
                return fs.unlinkSync(filePath);
            }
        };

        assert.throws(
            () => atomicReplaceFile(sourcePath, targetPath, fsSpy),
            /rename failed/
        );
        assert.ok(tempPath, 'temp path should be captured');
        assert.equal(fs.existsSync(tempPath), false);
    });
});
