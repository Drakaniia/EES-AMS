import { afterEach, describe, expect, it, vi } from 'vitest';
import { WorkerSqlDriver } from './client';

/**
 * The runtime driver's own tests.
 *
 * `client.ts` is the one `SqlDriver` the app actually uses, and every repo test
 * swaps in `NodeSqlDriver` instead - which has no gate and therefore cannot
 * reproduce what only the Worker driver does. The gate's job is to stop a stray
 * query landing between `BEGIN` and `COMMIT`; these tests are what keep the gate
 * from also stopping the transaction's *own* queries.
 */

type Posted = { id: number; op: string; sql?: string };

/** A Worker that answers every op, so the driver under test is exercised alone. */
class FakeWorker {
	static posted: Posted[] = [];

	onmessage: ((event: MessageEvent) => void) | null = null;

	postMessage(data: Posted): void {
		FakeWorker.posted.push({ id: data.id, op: data.op, sql: data.sql });
		queueMicrotask(() => {
			this.onmessage?.({
				data: { id: data.id, ok: true, value: answerFor(data.op, data.sql) }
			} as MessageEvent);
		});
	}

	terminate(): void {}
}

/** Enough SQL to look migrated, so these tests measure the gate and nothing else. */
function answerFor(op: string, sql: string | undefined): unknown {
	if (op === 'open-temporary') return 'memory';
	if (op === 'query') return sql?.includes('user_version') ? [{ user_version: 25 }] : [];
	if (op === 'execute') return 0;
	return null;
}

const realWorker = globalThis.Worker;

function withFakeWorker(): void {
	FakeWorker.posted = [];
	vi.stubGlobal('Worker', FakeWorker);
}

/** Resolve, or fail the test rather than hang it - a deadlock must not hang CI. */
async function within<T>(ms: number, run: () => Promise<T>): Promise<T> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	const expiry = new Promise<never>((_, reject) => {
		timer = setTimeout(() => reject(new Error(`still running after ${ms}ms`)), ms);
	});
	try {
		return await Promise.race([run(), expiry]);
	} finally {
		clearTimeout(timer);
	}
}

/** A Worker whose first `open` fails, so Retry is exercised for real. */
class FailOnceOpenWorker extends FakeWorker {
	static openCalls = 0;

	override postMessage(data: Posted): void {
		if (data.op === 'open') {
			FailOnceOpenWorker.openCalls += 1;
			if (FailOnceOpenWorker.openCalls === 1) {
				const id = data.id;
				queueMicrotask(() => {
					this.onmessage?.({
						data: {
							id,
							ok: false,
							error: { kind: 'Database', detail: 'could not open OPFS database (test)' }
						}
					} as MessageEvent);
				});
				return;
			}
		}
		super.postMessage(data);
	}

	override terminate(): void {}
}

afterEach(() => {
	vi.unstubAllGlobals();
	if (realWorker !== undefined) globalThis.Worker = realWorker;
});

describe('the gate on the worker driver', () => {
	it("runs the transaction's own queries instead of deadlocking on itself", async () => {
		withFakeWorker();
		const driver = new WorkerSqlDriver();
		try {
			await within(2000, () =>
				driver.transaction(async () => {
					await driver.execute('UPDATE students SET name = ? WHERE id = ?', ['a', 'b']);
				})
			);
			expect(FakeWorker.posted.map((m) => m.sql)).toContain(
				'UPDATE students SET name = ? WHERE id = ?'
			);
		} finally {
			await driver.close().catch(() => {});
		}
	});

	it('brackets the body in BEGIN and COMMIT', async () => {
		withFakeWorker();
		const driver = new WorkerSqlDriver();
		try {
			await within(2000, () =>
				driver.transaction(async () => {
					await driver.execute('UPDATE students SET name = ?', ['a']);
				})
			);
			const sql = FakeWorker.posted.map((m) => m.sql);
			expect(sql).toContain('BEGIN');
			expect(sql).toContain('COMMIT');
		} finally {
			await driver.close().catch(() => {});
		}
	});

	it('rolls back and rethrows when the body throws', async () => {
		withFakeWorker();
		const driver = new WorkerSqlDriver();
		try {
			await within(2000, async () => {
				await expect(
					driver.transaction(async () => {
						await driver.execute('UPDATE students SET name = ?', ['a']);
						throw new Error('boom');
					})
				).rejects.toThrow('boom');
			});
			const sql = FakeWorker.posted.map((m) => m.sql);
			expect(sql).toContain('ROLLBACK');
			expect(sql).not.toContain('COMMIT');
		} finally {
			await driver.close().catch(() => {});
		}
	});

	it('re-opens after a failed open instead of replaying the cached failure', async () => {
		FailOnceOpenWorker.openCalls = 0;
		FakeWorker.posted = [];
		vi.stubGlobal('Worker', FailOnceOpenWorker);
		const driver = new WorkerSqlDriver();
		try {
			await within(2000, async () => {
				await expect(driver.open()).rejects.toMatchObject({ kind: 'Database' });
				await driver.open();
			});
			expect(FailOnceOpenWorker.openCalls).toBe(2);
		} finally {
			await driver.close().catch(() => {});
		}
	});

	it('opens a temporary database without touching OPFS', async () => {
		withFakeWorker();
		const driver = new WorkerSqlDriver();
		try {
			const mode = await within(2000, () => driver.openTemporary());
			expect(mode).toBe('memory');
			await within(2000, () => driver.query('SELECT 1'));
			const ops = FakeWorker.posted.map((m) => m.op);
			expect(ops).toContain('open-temporary');
			expect(ops).not.toContain('open');
		} finally {
			await driver.close().catch(() => {});
		}
	});

	it('recovers back to OPFS after temporary mode', async () => {
		withFakeWorker();
		const driver = new WorkerSqlDriver();
		try {
			await within(2000, () => driver.openTemporary());
			FakeWorker.posted = [];
			await within(2000, () => driver.recover());
			const ops = FakeWorker.posted.map((m) => m.op);
			expect(ops).toContain('close');
			expect(ops).toContain('open');
		} finally {
			await driver.close().catch(() => {});
		}
	});
});
