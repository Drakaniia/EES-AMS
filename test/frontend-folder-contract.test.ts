import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import test from 'node:test';

const sourceRoots = ['src/lib', 'src/routes'];
const checkedExtensions = new Set(['.svelte', '.ts', '.js']);
const forbiddenImportPatterns = [
	{
		pattern: /['"]src\/components\/ui\//,
		message: 'shared UI must not import from src/components/ui'
	},
	{
		pattern: /['"](?:\.\.\/)+components\/ui\//,
		message: 'shared UI must be imported with $lib/components/ui/...'
	}
];

function sourceFiles(root: string): string[] {
	return readdirSync(root).flatMap((entry) => {
		const path = join(root, entry);
		const stats = statSync(path);
		if (stats.isDirectory()) return sourceFiles(path);
		const extension = path.slice(path.lastIndexOf('.'));
		return checkedExtensions.has(extension) ? [path] : [];
	});
}

test('frontend shared UI imports use the single $lib components root', () => {
	const files = sourceRoots.flatMap(sourceFiles);

	for (const file of files) {
		const source = readFileSync(file, 'utf8');
		for (const forbidden of forbiddenImportPatterns) {
			assert.doesNotMatch(
				source,
				forbidden.pattern,
				`${forbidden.message}: ${relative('.', file)}`
			);
		}
	}
});

test('legacy src/components/ui root is not used for shared UI source', () => {
	const legacyRoot = 'src/components/ui';
	if (!existsSync(legacyRoot)) return;

	const files = sourceFiles(legacyRoot);
	assert.deepEqual(files, []);
});
