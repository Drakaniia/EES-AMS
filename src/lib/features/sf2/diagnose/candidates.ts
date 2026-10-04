/**
 * Which files the diagnostic opens.
 *
 * Two rules, both of which exist because getting them wrong hides marks that are really
 * there:
 *
 * - **Every file in the workbook directory is a candidate, not just the one the database
 *   points at.** A month whose marks sit in a working copy the database no longer
 *   references is a month the app cannot see, and reporting it as `NoSheet` is wrong in
 *   the way that matters.
 * - **The list is not filtered on the extension.** Two of the four files in a real
 *   install's workbook directory are older per-class working copies named after their
 *   template id with no extension at all, and one of them holds the only `AUGUST` sheet
 *   in the directory. A file that is not a workbook fails to open and is reported as
 *   unreadable, which is the truth about it.
 *
 * The candidate list is sorted after the referenced file, and the referenced file is
 * probed first, so two runs over the same data produce the same report in the same order.
 */

import { getFileSystem } from '$lib/platform/fs';
import { LEGACY_WORKBOOK_DIR, getSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { anchorTemplate, type DbSnapshot } from './db-read';
import { probeWorkbook, type RawWorkbook } from './probe';
import type { WorkbookProbe } from './report';

/** The workbook the database's own template row points at. */
export function referencedWorkbookPath(snapshot: DbSnapshot): string | undefined {
	const path = anchorTemplate(snapshot)?.sourcePath ?? snapshot.monthTemplates[0]?.sourcePath ?? '';
	return path === '' ? undefined : path;
}

/** Whether `path` names a file that is there. */
export async function isFile(path: string): Promise<boolean> {
	const stat = await getFileSystem()
		.stat(path)
		.catch(() => undefined);
	return stat?.isFile === true;
}

/**
 * The directory every workbook is looked for in.
 *
 * The workbook the database points at has the app's own workbook folder as its parent,
 * which is where D13 puts them. When the database names no file at all, the app's own
 * accessor answers - which ensures the folder exists, a directory creation and nothing
 * more. Noted because this module otherwise writes nothing: it is the one call on this
 * path that touches the disk, and it creates the folder the app creates at startup
 * regardless.
 */
export async function resolveWorkbookDir(snapshot: DbSnapshot): Promise<string> {
	const referenced = referencedWorkbookPath(snapshot);
	if (referenced === undefined) return getSf2WorkbookDir();
	const separator = Math.max(referenced.lastIndexOf('/'), referenced.lastIndexOf('\\'));
	return separator < 0 ? getSf2WorkbookDir() : referenced.slice(0, separator);
}

/**
 * Every file worth opening, the referenced one first.
 *
 * The referenced file is only a candidate when it is on disk. Probing a path that is not
 * there would report a missing workbook as an unreadable one, and every month it should
 * have covered would become `ExcelUnavailable` instead of `NoSheet`.
 */
export async function workbookCandidates(
	referenced: string | undefined,
	workbookDir: string
): Promise<string[]> {
	const candidates: string[] = [];
	const seen = new Set<string>();
	if (referenced !== undefined && (await isFile(referenced))) {
		candidates.push(referenced);
		// Marked seen straight away: the referenced file is usually *also* in the directory
		// listing, and probing it twice would report it twice.
		seen.add(referenced);
	}
	// Sorted so the report reads the same way every run; the referenced file stays first.
	for (const path of (await listWorkbookFiles(workbookDir)).sort()) {
		if (seen.has(path)) continue;
		seen.add(path);
		candidates.push(path);
	}
	return candidates;
}

async function listWorkbookFiles(workbookDir: string): Promise<string[]> {
	const fileSystem = getFileSystem();
	const found: string[] = [];
	for (const dir of [workbookDir, `${workbookDir}/${LEGACY_WORKBOOK_DIR}`]) {
		found.push(...(await readDirectory(fileSystem, dir)));
	}
	return found;
}

async function readDirectory(
	fileSystem: ReturnType<typeof getFileSystem>,
	dir: string
): Promise<string[]> {
	try {
		const names = await fileSystem.readDir(dir);
		const found: string[] = [];
		for (const name of names) {
			const path = `${dir}/${name}`;
			if (await isFile(path)) found.push(path);
		}
		return found;
	} catch {
		// A directory that is not there is a normal state, not a failure.
		return [];
	}
}

/**
 * Read every candidate workbook.
 *
 * A workbook that will not open does not stop the others: its failure is kept as the
 * `ExcelUnavailable` reason for the months it would have covered, and the rest of the
 * diagnostic carries on. That is the whole difference between "the user has the file open
 * in Excel" and a failed tool.
 */
export async function probeAll(candidates: readonly string[]): Promise<WorkbookProbe[]> {
	const probes: WorkbookProbe[] = [];
	for (const path of candidates) {
		try {
			const workbook: RawWorkbook = await probeWorkbook(path);
			probes.push({ path, workbook });
		} catch (thrown) {
			probes.push({ path, readError: thrown instanceof Error ? thrown.message : String(thrown) });
		}
	}
	return probes;
}
