#!/usr/bin/env node
/**
 * Minimal build cache helpers for "build only when inputs change" behavior.
 *
 * Cache strategy:
 * - Collect relevant input files (filtered + excluding generated dirs)
 * - Fingerprint = sha256 over (relativePath, mtimeMs, size)
 * - Store cache as JSON in vscode-extension/.cache/
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

function ensureDir(dirPath) {
    if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true });
    }
}

function readJsonIfExists(filePath) {
    try {
        if (!fs.existsSync(filePath)) {
            return null;
        }
        const raw = fs.readFileSync(filePath, 'utf8');
        return JSON.parse(raw);
    } catch {
        return null;
    }
}

function writeJson(filePath, data) {
    ensureDir(path.dirname(filePath));
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n', 'utf8');
}

function walkFiles(rootPath, opts) {
    const {
        projectRoot,
        excludeDirNames,
        includeFile,
    } = opts;

    const results = [];

    function walk(currentPath) {
        let stats;
        try {
            stats = fs.statSync(currentPath);
        } catch {
            return;
        }

        if (stats.isDirectory()) {
            const name = path.basename(currentPath);
            if (excludeDirNames && excludeDirNames.has(name)) {
                return;
            }

            let entries;
            try {
                entries = fs.readdirSync(currentPath, { withFileTypes: true });
            } catch {
                return;
            }

            for (const entry of entries) {
                walk(path.join(currentPath, entry.name));
            }
            return;
        }

        if (!stats.isFile()) {
            return;
        }

        const relPath = projectRoot ? path.relative(projectRoot, currentPath) : currentPath;
        if (includeFile && !includeFile(relPath)) {
            return;
        }

        results.push({
            relPath,
            mtimeMs: stats.mtimeMs,
            size: stats.size,
        });
    }

    walk(rootPath);
    return results;
}

function fingerprintEntries(entries) {
    const hash = crypto.createHash('sha256');
    const sorted = [...entries].sort((a, b) => a.relPath.localeCompare(b.relPath));
    for (const e of sorted) {
        hash.update(e.relPath);
        hash.update('\0');
        hash.update(String(Math.floor(e.mtimeMs)));
        hash.update('\0');
        hash.update(String(e.size));
        hash.update('\n');
    }
    return hash.digest('hex');
}

function summarizeEntries(entries) {
    let maxMtimeMs = 0;
    for (const e of entries) {
        if (e.mtimeMs > maxMtimeMs) {
            maxMtimeMs = e.mtimeMs;
        }
    }
    return {
        fileCount: entries.length,
        maxMtimeMs,
        fingerprint: fingerprintEntries(entries),
    };
}

function summarizePaths(roots, opts) {
    const all = [];
    for (const rootPath of roots) {
        if (!rootPath) {
            continue;
        }
        if (!fs.existsSync(rootPath)) {
            continue;
        }
        const stats = fs.statSync(rootPath);
        if (stats.isDirectory()) {
            all.push(...walkFiles(rootPath, opts));
        } else if (stats.isFile()) {
            const relPath = opts.projectRoot ? path.relative(opts.projectRoot, rootPath) : rootPath;
            if (!opts.includeFile || opts.includeFile(relPath)) {
                all.push({
                    relPath,
                    mtimeMs: stats.mtimeMs,
                    size: stats.size,
                });
            }
        }
    }
    return summarizeEntries(all);
}

function getMaxMtimeInDir(dirPath) {
    if (!fs.existsSync(dirPath)) {
        return 0;
    }
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    let maxMtimeMs = 0;
    for (const entry of entries) {
        const full = path.join(dirPath, entry.name);
        let stats;
        try {
            stats = fs.statSync(full);
        } catch {
            continue;
        }
        if (entry.isDirectory()) {
            const nested = getMaxMtimeInDir(full);
            if (nested > maxMtimeMs) {
                maxMtimeMs = nested;
            }
        } else if (entry.isFile()) {
            if (stats.mtimeMs > maxMtimeMs) {
                maxMtimeMs = stats.mtimeMs;
            }
        }
    }
    return maxMtimeMs;
}

module.exports = {
    ensureDir,
    readJsonIfExists,
    writeJson,
    summarizePaths,
    getMaxMtimeInDir,
};

