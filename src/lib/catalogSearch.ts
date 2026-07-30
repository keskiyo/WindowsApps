import type { AppInfo } from '../types'
import { isWithinOneEdit, queryTokenVariants } from './searchQueryVariants'

const MIN_FUZZY_LENGTH = 4

interface SearchFields {
	name: string
	product: string
	publisher: string
	secondary: string
	aliases: string
	words: string[]
}

interface QueryToken {
	value: string
	variants: string[]
}

const searchFields = new WeakMap<AppInfo, SearchFields>()

function isGenuineWindowsTerminal(app: AppInfo): boolean {
	return (
		app.launchKind === 'app_user_model_id' &&
		app.path
			.toLocaleLowerCase()
			.startsWith('microsoft.windowsterminal_8wekyb3d8bbwe!')
	)
}

function fieldsFor(app: AppInfo): SearchFields {
	const cached = searchFields.get(app)
	if (cached) return cached
	const name = app.name.toLocaleLowerCase()
	const product = (app.productName ?? '').toLocaleLowerCase()
	const fields = {
		name,
		product,
		publisher: (app.publisher ?? '').toLocaleLowerCase(),
		aliases: isGenuineWindowsTerminal(app) ? 'cmd' : '',
		secondary: [
			app.path,
			app.installLocation,
			app.originalFilename,
			app.description,
			app.visibilityReasons?.join(' '),
			app.visibilityReasons?.includes('product_component')
				? 'helper service component'
				: null,
			app.visibilityReasons?.includes('documentation_shortcut')
				? 'documentation docs'
				: null,
		]
			.filter(Boolean)
			.join(' ')
			.toLocaleLowerCase(),
		words: `${name} ${product}`
			.split(/[^\p{L}\p{N}]+/u)
			.filter(word => word.length >= MIN_FUZZY_LENGTH),
	}
	searchFields.set(app, fields)
	return fields
}

function directScore(fields: SearchFields, token: string): number {
	if (fields.name === token) return 100
	if (fields.name.startsWith(token)) return 90
	if (fields.aliases === token) return 80
	if (
		fields.name
			.split(/[\s\-_.()[\]]+/)
			.some(word => word.startsWith(token))
	)
		return 70
	if (fields.name.includes(token) || fields.product.includes(token))
		return 50
	if (fields.publisher.includes(token)) return 30
	if (token.length >= 3 && fields.secondary.includes(token)) return 10
	return 0
}

function tokenScore(fields: SearchFields, token: QueryToken): number {
	for (let index = 0; index < token.variants.length; index += 1) {
		const score = directScore(fields, token.variants[index])
		if (score > 0) return score + (index === 0 ? 200 : 100)
	}
	if (token.value.length < MIN_FUZZY_LENGTH) return 0
	for (let index = 0; index < token.variants.length; index += 1)
		if (
			fields.words.some(word =>
				isWithinOneEdit(token.variants[index], word),
			)
		)
			return index === 0 ? 50 : 40
	return 0
}

function scoreApp(fields: SearchFields, tokens: QueryToken[]): number {
	let total = 0
	for (const token of tokens) {
		const score = tokenScore(fields, token)
		if (score === 0) return 0
		total += score
	}
	return total
}

function queryTokens(query: string): QueryToken[] {
	return query
		.trim()
		.toLocaleLowerCase()
		.split(/\s+/)
		.filter(Boolean)
		.map(value => ({ value, variants: queryTokenVariants(value) }))
}

function queryCandidates(apps: AppInfo[], tokens: QueryToken[]): AppInfo[] {
	const terminalOnly =
		tokens.length === 1 && tokens[0].variants.includes('cmd')
	return terminalOnly ? apps.filter(isGenuineWindowsTerminal) : apps
}

export function filterAppsByQuery(apps: AppInfo[], query: string): AppInfo[] {
	const tokens = queryTokens(query)
	if (tokens.length === 0) return apps
	return queryCandidates(apps, tokens).filter(
		app => scoreApp(fieldsFor(app), tokens) > 0,
	)
}

export function rankAppsByQuery(apps: AppInfo[], query: string): AppInfo[] {
	const tokens = queryTokens(query)
	if (tokens.length === 0) return apps
	return queryCandidates(apps, tokens)
		.map(app => ({ app, score: scoreApp(fieldsFor(app), tokens) }))
		.filter(entry => entry.score > 0)
		.sort(
			(a, b) =>
				b.score - a.score ||
				a.app.name.length - b.app.name.length ||
				a.app.name.localeCompare(b.app.name),
		)
		.map(entry => entry.app)
}
