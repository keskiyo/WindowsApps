/** @type {import('prettier').Config & import('prettier-plugin-tailwindcss').PluginOptions} */
const config = {
	semi: false,
	singleQuote: true,
	jsxSingleQuote: false,
	arrowParens: 'avoid',

	// Отступы
	useTabs: true,
	tabWidth: 4,

	// Tailwind CSS v4
	tailwindStylesheet: './src/app/styles/index.css',
	tailwindFunctions: ['cn', 'clsx', 'cva', 'cx', 'tw', 'twMerge'],

	plugins: ['prettier-plugin-tailwindcss'],
}

export default config
