import js from '@eslint/js'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const componentColorPattern = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i
const localRules = {
	rules: {
		'no-component-color-literals': {
			meta: {
				type: 'problem',
				schema: [],
				messages: {
					literal:
						'Component-local colors are forbidden; use a token from src/index.css.',
				},
			},
			create(context) {
				const check = node => {
					const value =
						node.type === 'Literal' ? node.value : node.value.raw
					if (
						typeof value === 'string' &&
						componentColorPattern.test(value)
					) {
						context.report({ node, messageId: 'literal' })
					}
				}
				return {
					Literal: check,
					TemplateElement: check,
				}
			},
		},
	},
}

export default tseslint.config(
	{
		ignores: ['dist', 'coverage', 'graphify-out', 'node_modules', 'src-tauri/target'],
	},
	{
		files: ['src/**/*.{ts,tsx}'],
		extends: [
			js.configs.recommended,
			...tseslint.configs.recommended,
		],
		languageOptions: {
			ecmaVersion: 2020,
			globals: {
				...globals.browser,
				...globals.es2021,
			},
		},
		rules: {
			'@typescript-eslint/no-unused-vars': [
				'error',
				{ argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
			],
			'@typescript-eslint/no-explicit-any': 'error',
			// Release builds register no log sink, so console output is a dead end;
			// user-facing problems belong in the UI. See src/AGENTS_frontend.md.
			'no-console': 'error',
			'react-hooks/rules-of-hooks': 'error',
			'react-hooks/exhaustive-deps': 'error',
			'react-refresh/only-export-components': [
				'error',
				{ allowConstantExport: true },
			],
		},
		plugins: {
			'react-hooks': reactHooks,
			'react-refresh': reactRefresh,
		},
	},
	{
		// Architectural boundary, not a preference: the Tauri runtime is reached through
		// the typed clients in src/lib or a hook in src/hooks, so presentation and state
		// stay testable without the desktop runtime. See AGENTS.md §5.
		files: ['src/components/**/*.{ts,tsx}', 'src/store/**/*.{ts,tsx}'],
		plugins: {
			local: localRules,
		},
		rules: {
			'local/no-component-color-literals': 'error',
			'no-restricted-imports': [
				'error',
				{
					patterns: [
						{
							group: ['@tauri-apps/*', '@tauri-apps/**'],
							message:
								'Import the typed client from src/lib (tauri.ts, system.ts) or wrap the API in a hook under src/hooks.',
						},
					],
				},
			],
		},
	},
)
