/** @type {import('prettier').Config & import('prettier-plugin-tailwindcss').PluginOptions} */
const config = {
	semi: false,
	singleQuote: true,
	jsxSingleQuote: false,
	arrowParens: 'avoid',

	// Отступы
	useTabs: true,
	tabWidth: 4,

	// core.autocrlf=true возвращает CRLF при каждом checkout на Windows, а
	// значение по умолчанию 'lf' роняло бы на этом format:check у всех локально,
	// проходя при этом на Linux-раннере.
	endOfLine: 'auto',

	// Tailwind CSS v4
	tailwindStylesheet: './src/app/styles/index.css',
	tailwindFunctions: ['cn', 'clsx', 'cva', 'cx', 'tw', 'twMerge'],

	plugins: ['prettier-plugin-tailwindcss'],
}

export default config
