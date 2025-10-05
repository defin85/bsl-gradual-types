// esbuild bundler configuration for VSCode extension

const esbuild = require('esbuild');

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');

/**
 * @type {import('esbuild').Plugin}
 */
const esbuildProblemMatcherPlugin = {
    name: 'esbuild-problem-matcher',

    setup(build) {
        build.onStart(() => {
            console.log('[watch] build started');
        });
        build.onEnd(result => {
            result.errors.forEach(({ text, location }) => {
                console.error(`✘ [ERROR] ${text}`);
                console.error(`    ${location.file}:${location.line}:${location.column}:`);
            });
            console.log('[watch] build finished');
        });
    }
};

async function main() {
    const ctx = await esbuild.context({
        entryPoints: ['src/extension.ts'],
        bundle: true,
        format: 'cjs',
        minify: production,
        sourcemap: !production,
        sourcesContent: false,
        platform: 'node',
        outfile: 'out/extension.js',
        external: ['vscode'],
        logLevel: 'silent',
        plugins: [
            esbuildProblemMatcherPlugin
        ],
        treeShaking: true,
        metafile: production,
    });

    if (watch) {
        await ctx.watch();
    } else {
        await ctx.rebuild();
        await ctx.dispose();

        // Вывод статистики в production режиме
        if (production) {
            const result = await esbuild.build({
                entryPoints: ['src/extension.ts'],
                bundle: true,
                format: 'cjs',
                minify: true,
                sourcemap: false,
                platform: 'node',
                outfile: 'out/extension.js',
                external: ['vscode'],
                treeShaking: true,
                metafile: true,
            });

            if (result.metafile) {
                const text = await esbuild.analyzeMetafile(result.metafile, {
                    verbose: false,
                });
                console.log('\n📊 Bundle Analysis:');
                console.log(text);
            }
        }
    }
}

main().catch(e => {
    console.error(e);
    process.exit(1);
});
