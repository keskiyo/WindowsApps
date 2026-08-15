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
						'Component-local colors are forbidden; use a token from src/app/styles/index.css.',
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
		ignores: [
			'dist',
			'coverage',
			'graphify-out',
			'node_modules',
			'src-tauri/target',
		],
	},
	{
		files: ['src/**/*.{ts,tsx}', 'tests/**/*.{ts,tsx}'],
		extends: [js.configs.recommended, ...tseslint.configs.recommended],
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
		// Architectural boundary, not a preference: the Tauri runtime is reached only through
		// the four integration modules listed in `ignores`, so presentation, state, entity
		// logic and shared utilities stay testable without the desktop runtime.
		// See AGENTS.md §4 and src/AGENTS.md §1.
		files: ['src/**/*.{ts,tsx}'],
		ignores: [
			'src/shared/api/tauri/client.ts',
			'src/shared/platform/window/useWindowControls.ts',
			'src/entities/system/api/systemClient.ts',
			'src/features/update-app/model/useUpdater.ts',
		],
		rules: {
			'no-restricted-imports': [
				'error',
				{
					patterns: [
						{
							group: ['@tauri-apps/*', '@tauri-apps/**'],
							message:
								'Reach the runtime through shared/api/tauri, a typed entity client (entities/*/api) or an owning integration hook; components, stores, entity logic and shared code never import Tauri directly.',
						},
					],
				},
			],
		},
	},
	{
		// Colors come from the token layer in src/app/styles/index.css, never from a literal
		// next to the markup that uses it.
		files: ['src/**/*.{ts,tsx}'],
		plugins: {
			local: localRules,
		},
		rules: {
			'local/no-component-color-literals': 'error',
		},
	},
)
