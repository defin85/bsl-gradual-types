import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../media/webview',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        typeDetails: path.resolve(__dirname, 'src/typeDetails.tsx'),
        quickActions: path.resolve(__dirname, 'src/quickActions.tsx'),
      },
      output: {
        entryFileNames: '[name].js',
        chunkFileNames: '[name].js',
        assetFileNames: '[name].[ext]'
      }
    }
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
