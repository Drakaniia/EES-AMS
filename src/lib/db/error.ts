/**
 * AppError.
 *
 * The wording is part of the UI contract: `errorMessage()` reproduces the message
 * strings byte-for-byte and must not be reworded.
 */

type AppErrorKind =
	| 'Database'
	| 'Pool'
	| 'StudentNotFound'
	| 'EventNotFound'
	| 'ClassNotFound'
	| 'DuplicateAttendance'
	| 'InvalidInput'
	| 'Internal';

export type AppError = {
	kind: AppErrorKind;
	/** The `{0}` payload from the Rust `#[error(...)]` attribute. */
	detail: string;
};

/** Mirrors the `#[error("...")]` templates, in declaration order. */
const MESSAGE_TEMPLATES: Record<AppErrorKind, string> = {
	Database: 'database error: {detail}',
	Pool: 'connection pool error: {detail}',
	StudentNotFound: 'student not found: {detail}',
	EventNotFound: 'event not found: {detail}',
	ClassNotFound: 'class not found: {detail}',
	DuplicateAttendance: 'duplicate attendance: {detail}',
	InvalidInput: 'invalid input: {detail}',
	Internal: 'internal server error: {detail}'
};

export function appError(kind: AppErrorKind, detail: string): AppError {
	return { kind, detail };
}

export function invalidInput(detail: string): AppError {
	return appError('InvalidInput', detail);
}

export function internal(detail: string): AppError {
	return appError('Internal', detail);
}

/** The string the teacher sees — identical to what Rust's `Display` produced. */
export function errorMessage(error: AppError): string {
	return MESSAGE_TEMPLATES[error.kind].replace('{detail}', error.detail);
}

/** True for the plain `{ kind, detail }` objects repos throw instead of `Error`s. */
function isAppError(thrown: unknown): thrown is AppError {
	return typeof thrown === 'object' && thrown !== null && 'kind' in thrown && 'detail' in thrown;
}

/**
 * The message to show a teacher for anything the data layer threw.
 *
 * Repos throw plain `AppError` objects, which are not `Error`s -- so the common
 * `thrown instanceof Error ? thrown.message : fallback` shape turns *every*
 * database and validation failure into its fallback, and the real reason is
 * never shown. This unwraps all three shapes this layer produces: `Error`,
 * `AppError`, and `string`. An `AppError` goes through {@link errorMessage}, so
 * the displayed text stays the byte-for-byte Rust contract.
 */
export function describeError(thrown: unknown, fallback: string): string {
	if (thrown instanceof Error) return thrown.message;
	if (typeof thrown === 'string') return thrown;
	if (isAppError(thrown)) return errorMessage(thrown);
	return fallback;
}

/** Wrap anything thrown into an `AppError` without losing its message. */
export function asAppError(thrown: unknown): AppError {
	if (isAppError(thrown)) return thrown;
	const detail =
		thrown instanceof Error ? thrown.message : typeof thrown === 'string' ? thrown : String(thrown);
	return internal(detail);
}
