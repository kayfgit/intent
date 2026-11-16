import { defineConfig } from 'vitest/config';

export default defineConfig({
  // Relative asset paths, so the built demo works from any folder.
  base: './',
  server: {
    port: 5173,
    strictPort: true,
    // The layout file lives one level up, in the repository's layouts folder.
    fs: { allow: ['..'] },
  },
  build: { target: 'es2022' },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
