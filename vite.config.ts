import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
	plugins: [react(), tailwindcss()],
	clearScreen: false,
	test: {
		environment: 'jsdom',
		setupFiles: './tests/frontend/setup.ts',
		// Reported, not enforced. A coverage percentage is not evidence that the critical paths
		// are covered, and a threshold rewards tests written to move the number — the gate that
		// actually protects behaviour is the requirement that every fixed bug gets a regression
		// test. The report exists so a reviewer can see what a change left untested.
		coverage: {
			provider: 'v8',
			reporter: ['text', 'lcov'],
			reportsDirectory: 'coverage',
			// Only frontend production sources. The Rust crate has its own colocated tests, and
			// test/build files would otherwise dilute the report into meaninglessness.
			include: ['src/**/*.{ts,tsx}'],
			exclude: ['src/**/types.ts', 'src/main.tsx', 'src/vite-env.d.ts'],
		},
	},
	server: {
		strictPort: true,
		host: '127.0.0.1',
	},
})
