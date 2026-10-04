import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';
import type { ServerResponse } from 'node:http';

/**
 * SharedArrayBuffer is gated on cross-origin isolation, and the `opfs` VFS
 * refuses to install without it (it needs Atomics to proxy OPFS's async file
 * API through the synchronous sqlite3_vfs API). Without these headers the VFS
 * install fails, its error is swallowed to a `warn`, and `oo1.OpfsDb` is left
 * undefined -- which surfaces much later as a confusing
 * "OpfsDb is not a constructor".
 *
 * This has to be a middleware rather than `server.headers`: Vite only applies
 * `server.headers` inside its own static-file and index.html middlewares, and
 * SvelteKit's dev handler answers the request before either of those run.
 * Release builds get the same headers from `app.security.headers` in
 * `src-tauri/tauri.conf.json`.
 */
function crossOriginIsolation(): Plugin {
	return {
		name: 'cross-origin-isolation',
		configureServer(server) {
			server.middlewares.use((_request, response: ServerResponse, next) => {
				response.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
				response.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
				next();
			});
		}
	};
}

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), crossOriginIsolation()],
	server: {
		host: '127.0.0.1',
		port: 1420,
		strictPort: true,
		// `workbook-files.ts` imports the bundled DepEd template from `src-tauri`
		// with `?url`, which dev serves as an `/@fs/` request. Without this the
		// dev server 403s it and every "create workbook from the bundled template"
		// fails, while the release build (where the file becomes a hashed asset)
		// works fine.
		fs: { allow: ['src-tauri'] }
	}
});
