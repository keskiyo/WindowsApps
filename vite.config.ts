import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
	plugins: [react(), tailwindcss()],
	clearScreen: false,
	test: {
		environment: 'jsdom',
		setupFiles: './src/test/setup.ts',
	},
	server: {
		strictPort: true,
		host: '127.0.0.1',
	},
})
