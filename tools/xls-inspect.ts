/**
 * Dev-time workbook inspection — the SheetJS (`xlsx`) tooling the migration
 * spec asked for (D6, §5.2: "SheetJS appears only in `tools/` … Never bundled").
 *
 * ExcelJS is the app's only runtime workbook engine. This script is the dev-only
 * reader: it dumps a legacy `.xls` (or any `.xlsx`) to JSON so a workbook can be
 * inspected on a machine without Excel. Nothing under `src/` imports `xlsx`, so
 * none of it reaches the bundle; `bun run tools:xls` is its whole footprint.
 *
 * Usage:
 *   bun run tools:xls path/to/workbook.xls
 *   bun run tools:xls path/to/workbook.xlsx --sheet "JUNE 2025" --rows 30
 */

import { readFile } from 'node:fs/promises';
import * as XLSX from 'xlsx';

const HELP = `Usage: bun run tools:xls <workbook.xls|xlsx> [--sheet <name>] [--rows <n>]

Prints the workbook's sheet inventory as JSON, plus the first rows of one sheet.

Options:
  --sheet <name>  dump a single sheet instead of just the inventory
  --rows <n>      how many rows of that sheet to print (default: 15)
  --help          show this message`;

type CliOptions = {
	file: string;
	sheet: string | undefined;
	rows: number;
};

function parseArgs(argv: string[]): CliOptions {
	const options: CliOptions = { file: '', sheet: undefined, rows: 15 };
	for (let index = 0; index < argv.length; index += 1) {
		const argument = argv[index];
		if (argument === '--help') {
			console.log(HELP);
			process.exit(0);
		} else if (argument === '--sheet') {
			options.sheet = argv[++index];
		} else if (argument === '--rows') {
			options.rows = Number(argv[++index]);
		} else if (argument?.startsWith('--')) {
			throw new Error(`unknown option ${argument}`);
		} else if (!options.file && argument) {
			options.file = argument;
		} else {
			throw new Error(`unexpected argument ${argument}`);
		}
	}
	if (!options.file) throw new Error('no workbook given');
	if (!Number.isInteger(options.rows) || options.rows < 1) {
		throw new Error('--rows must be a positive integer');
	}
	return options;
}

async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	const workbook = XLSX.read(await readFile(options.file), { type: 'buffer' });

	const inventory = workbook.SheetNames.map((name) => {
		const sheet = workbook.Sheets[name];
		const range = sheet?.['!ref'];
		return { name, range: range ?? null };
	});
	const report: Record<string, unknown> = { file: options.file, sheets: inventory };

	if (options.sheet !== undefined) {
		const sheet = workbook.Sheets[options.sheet];
		if (!sheet) {
			throw new Error(
				`no sheet ${JSON.stringify(options.sheet)}; the workbook has ${workbook.SheetNames.join(', ')}`
			);
		}
		report.sheet = {
			name: options.sheet,
			rows: XLSX.utils.sheet_to_json<unknown[]>(sheet, {
				header: 1,
				defval: null,
				blankrows: false
			})
		};
	}

	console.log(JSON.stringify(report, null, 2));
}

main().catch((thrown: unknown) => {
	console.error(`xls-inspect: ${thrown instanceof Error ? thrown.message : String(thrown)}`);
	process.exitCode = 1;
});
