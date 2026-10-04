import type { SqlDriver } from './driver';
import { WorkerSqlDriver } from './client';

/**
 * The one accessor for the app's database.
 *
 * Repos call `getDriver()`; they never construct a driver. Tests inject the
 * Node driver with `useDriver()` before touching any repo.
 */

let injected: SqlDriver | null = null;
let shared: SqlDriver | null = null;

export function getDriver(): SqlDriver {
	if (injected) return injected;
	shared ??= new WorkerSqlDriver();
	return shared;
}

/** Test seam. Pass `null` to go back to the real Worker driver. */
export function useDriver(driver: SqlDriver | null): void {
	injected = driver;
}

export { WorkerSqlDriver } from './client';
export * from './driver';
export * from './error';
export type { WorkerRequest, WorkerResponse } from './protocol';
export { DB_FILENAME } from './protocol';
