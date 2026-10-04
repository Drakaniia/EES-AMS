/**
 * Raw SQL text, as imported by the migration runner.
 *
 * Vite serves `?raw` imports as the file's contents; TypeScript has no idea
 * that exists, so the shape is declared here rather than in every module.
 */
declare module '*.sql?raw' {
	const content: string;
	export default content;
}
