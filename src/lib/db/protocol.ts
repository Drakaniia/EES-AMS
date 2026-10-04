import type { AppError } from './error';
import type { SqlParam } from './driver';

/** Messages exchanged with `worker.ts`. Both sides import these types. */

export type WorkerRequest = {
	id: number;
	op: 'open' | 'query' | 'execute' | 'script' | 'export' | 'import' | 'close';
	sql?: string;
	params?: SqlParam[];
	/** `import` only: the bytes of a `.sqlite` file image to replace the live database with. */
	bytes?: Uint8Array;
};

export type WorkerResponse =
	| { id: number; ok: true; value: unknown }
	| { id: number; ok: false; error: AppError };

/** The OPFS file the app's database lives in. */
export const DB_FILENAME = 'ees-ams.sqlite3';
