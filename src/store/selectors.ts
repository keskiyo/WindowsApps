import { deduplicateVisibleApps } from '../lib/appDeduplication'
import type { AppInfo, AppView } from '../types'
import type { AppState } from './appStore'

export function filterVisibleApps(
	categorized: AppInfo[],
	activeView: AppView,
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): AppInfo[] {
	if (activeView === 'settings') return []
	// Set lookups, not Array.includes: this runs over the whole catalog on every
	// hydration patch, so a linear scan per app makes it O(apps × hidden).
	const hidden = new Set(hiddenAppIds)
	if (activeView === 'auxiliary')
		return categorized.filter(
			app => app.visibilityClass === 'auxiliary' && !hidden.has(app.id),
		)
	if (activeView === 'hidden')
		return categorized.filter(app => hidden.has(app.id))
	const visible = categorized.filter(
		app => app.visibilityClass !== 'auxiliary' && !hidden.has(app.id),
	)
	if (activeView !== 'favorites') return visible
	const favorites = new Set(favoriteAppIds)
	return visible.filter(app => favorites.has(app.id))
}

/**
 * Lowercased search text per record. Building it means concatenating ten fields and a
 * locale-aware lowercase, which is far too expensive to repeat for every app on every
 * keystroke. Caching is safe because the store never mutates app records in place —
 * `applyPatches` and `mergeIcon` build new objects, so hydrated metadata arrives under a
 * new key and is indexed fresh. A WeakMap keeps entries alive only as long as the records.
 */
const haystacks = new WeakMap<AppInfo, string>()

function searchHaystack(app: AppInfo): string {
	const cached = haystacks.get(app)
	if (cached !== undefined) return cached
	// Match across every searchable field (name, publisher, description, install path,
	// location) so nothing is missed.
	const haystack = [
		app.name,
		app.publisher,
		app.description,
		app.path,
		app.installLocation,
		app.productName,
		app.originalFilename,
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
		.toLocaleLowerCase()
	haystacks.set(app, haystack)
	return haystack
}

export function filterAppsByQuery(apps: AppInfo[], query: string): AppInfo[] {
	// Split into whitespace tokens so "world warcraft" matches "World of Warcraft" and
	// each fragment can hit a different field.
	const tokens = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean)
	if (tokens.length === 0) return apps
	return apps.filter(app => {
		const haystack = searchHaystack(app)
		return tokens.every(token => haystack.includes(token))
	})
}

/**
 * Search fields split by weight. A flat haystack made a match in the install path count as much
 * as a match in the name, so typing "tel" could bury Telegram under apps that merely live under a
 * "…\tel…" folder, and short common words matched every app through their path. Ranking scores a
 * name/prefix match far above a path match and drops path noise for short tokens.
 */
interface SearchFields {
	name: string
	product: string
	publisher: string
	secondary: string
}

const searchFields = new WeakMap<AppInfo, SearchFields>()

function fieldsFor(app: AppInfo): SearchFields {
	const cached = searchFields.get(app)
	if (cached) return cached
	const fields: SearchFields = {
		name: app.name.toLocaleLowerCase(),
		product: (app.productName ?? '').toLocaleLowerCase(),
		publisher: (app.publisher ?? '').toLocaleLowerCase(),
		secondary: [
			app.path,
			app.installLocation,
			app.originalFilename,
			app.description,
			app.visibilityReasons?.join(' '),
		]
			.filter(Boolean)
			.join(' ')
			.toLocaleLowerCase(),
	}
	searchFields.set(app, fields)
	return fields
}

/** Best match tier for one token; `0` means the token matched nothing (the app is excluded). */
function tokenScore(fields: SearchFields, token: string): number {
	const { name } = fields
	if (name === token) return 100
	if (name.startsWith(token)) return 90
	if (name.split(/[\s\-_.()[\]]+/).some(word => word.startsWith(token)))
		return 70
	if (name.includes(token) || fields.product.includes(token)) return 50
	if (fields.publisher.includes(token)) return 30
	// Path and other secondary fields only for tokens long enough not to be noise.
	if (token.length >= 3 && fields.secondary.includes(token)) return 10
	return 0
}

/**
 * Filter + rank by relevance. Every token must match (AND); each token contributes its best-field
 * tier; the app score is their sum, so stronger and more-complete matches rank higher. Ties break
 * toward the shorter (more exact) name. Returns the same app references — identity is preserved.
 */
export function rankAppsByQuery(apps: AppInfo[], query: string): AppInfo[] {
	const tokens = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean)
	if (tokens.length === 0) return apps
	const scored: Array<{ app: AppInfo; score: number }> = []
	for (const app of apps) {
		const fields = fieldsFor(app)
		let total = 0
		let matchedAll = true
		for (const token of tokens) {
			const score = tokenScore(fields, token)
			if (score === 0) {
				matchedAll = false
				break
			}
			total += score
		}
		if (matchedAll) scored.push({ app, score: total })
	}
	scored.sort(
		(a, b) =>
			b.score - a.score ||
			a.app.name.length - b.app.name.length ||
			a.app.name.localeCompare(b.app.name),
	)
	return scored.map(entry => entry.app)
}

type CategorizedAppsState = Pick<
	AppState,
	'apps' | 'categoryOverrides' | 'promotedAppIds' | 'promotedAppIdentities'
>

/**
 * Object identity is the point here, not just speed. Every `catalog://patches` event
 * replaces the apps array while icons stream in, and cloning every record would hand each
 * card a new `app` reference, defeating `memo(AppCard)` and re-rendering the whole grid
 * ~N/24 times during startup. Records that nothing applies to are returned untouched, so a
 * patch only invalidates the cards it actually changed.
 */
export function selectCategorizedApps(state: CategorizedAppsState): AppInfo[] {
	const promotedIds = new Set(state.promotedAppIds)
	const promotedIdentities = new Set(state.promotedAppIdentities)
	return deduplicateVisibleApps(
		state.apps.map(app => {
			const category = state.categoryOverrides[app.id] ?? app.category
			const promote =
				app.visibilityClass === 'auxiliary' &&
				(promotedIds.has(app.id) ||
					promotedIdentities.has(app.canonicalIdentity ?? app.id))
			if (category === app.category && !promote) return app
			const categorized = { ...app, category }
			return promote
				? {
						...categorized,
						visibilityClass: 'primary' as const,
						userPromoted: true,
					}
				: categorized
		}),
	)
}

export function selectVisibleApps(state: AppState): AppInfo[] {
	return filterVisibleApps(
		selectCategorizedApps(state),
		state.activeView,
		state.hiddenAppIds,
		state.favoriteAppIds,
	)
}

export function selectFilteredApps(state: AppState): AppInfo[] {
	return filterAppsByQuery(selectVisibleApps(state), state.query)
}
