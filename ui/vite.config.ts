import { defineConfig } from 'vitest/config';

// Tauri loads the dev server at a fixed port (devUrl in src-tauri/tauri.conf.json).
export default defineConfig({
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { outDir: 'dist', emptyOutDir: true },
  test: { include: ['src/**/*.test.ts'], environment: 'node' }
});
