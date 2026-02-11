import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
  ],
  
  // Tauri expects a relative base path
  base: './',
  
  // Prevent vite from obscuring rust errors
  clearScreen: false,
  
  // Tauri uses a fixed port for development
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Tell vite to ignore watching server directory
      ignored: ['**/server/**'],
    },
  },
  
  // Build configuration for Tauri
  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    // Don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
