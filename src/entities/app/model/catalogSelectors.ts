import { deduplicateVisibleApps } from '../lib/appDeduplication'
import { appIdentity } from '../lib/appIdentity'
import {
	INSTALLERS_DOCS_CATEGORY,
	isCatalogArtifact,
} from '../lib/catalogArtifacts'
import { filterAppsByQuery } from '../lib/catalogSearch'
import type { AppInfo, AppView } from './app.types'
import type { AppCategory } from '../../category'

export type AppPredicate = (app: AppInfo) => boolean

export function createMarkLookup(
	ids: string[],
	identities: string[],
): AppPredicate {
	const byId = new Set(ids)
	const byIdentity = new Set(identities)
	return app => byId.has(app.id) || byIdentity.has(appIdentity(app))
}

export function filterVisibleApps(
	categorized: AppInfo[],
	activeView: AppView,
	isHidden: AppPredicate,
	isFavorite: AppPredicate,
): AppInfo[] {
	if (
		activeView === 'settings' ||
		activeView === 'more' ||
		activeView === 'scenarios'
	)
		return []
	if (activeView === 'installers_docs')
		return categorized.filter(
			app => isCatalogArtifact(app) && !isHidden(app),
		)
	if (activeView === 'auxiliary')
		return categorized.filter(
			app =>
				!isCatalogArtifact(app) &&
				app.visibilityClass === 'auxiliary' &&
				!isHidden(app),
		)
	if (activeView === 'hidden') return categorized.filter(isHidden)
	const visible = categorized.filter(
		app =>
			!isCatalogArtifact(app) &&
			app.visibilityClass !== 'auxiliary' &&
			!isHidden(app),
	)
	return activeView === 'favorites' ? visible.filter(isFavorite) : visible
}

export interface SearchScopeCounts {
	auxiliary: number
	hidden: number
	installersDocs: number
}

const NO_SCOPE_MATCHES: SearchScopeCounts = {
	auxiliary: 0,
	hidden: 0,
	installersDocs: 0,
}

export function selectSearchScopeCounts(
	categorized: AppInfo[],
	query: string,
	hiddenAppIds: string[],
): SearchScopeCounts {
	if (!query.trim()) return NO_SCOPE_MATCHES
	const hidden = new Set(hiddenAppIds)
	const buckets: Record<keyof SearchScopeCounts, AppInfo[]> = {
		auxiliary: [],
		hidden: [],
		installersDocs: [],
	}
	for (const app of categorized) {
		if (hidden.has(app.id)) buckets.hidden.push(app)
		else if (isCatalogArtifact(app)) buckets.installersDocs.push(app)
		else if (app.visibilityClass === 'auxiliary')
			buckets.auxiliary.push(app)
	}
	return {
		auxiliary: filterAppsByQuery(buckets.auxiliary, query).length,
		hidden: filterAppsByQuery(buckets.hidden, query).length,
		installersDocs: filterAppsByQuery(buckets.installersDocs, query).length,
	}
}

export function selectRecentApps(
	apps: AppInfo[],
	rankOf: (app: AppInfo) => number,
	limit: number,
): AppInfo[] {
	if (apps.length <= 1) return apps.slice(0, limit)
	return [...apps]
		.sort((left, right) => {
			const difference = rankOf(right) - rankOf(left)
			return difference === 0
				? left.name.localeCompare(right.name)
				: difference
		})
		.slice(0, limit)
}

export function selectUnclassifiedApps(apps: AppInfo[]): AppInfo[] {
	return apps
		.filter(app => app.category === 'other')
		.sort((left, right) => left.name.localeCompare(right.name))
}

export interface CatalogCounts {
	visibleCategorizedApps: AppInfo[]
	navigationCounts: Map<string, number>
	favoriteCount: number
	hiddenCount: number
	auxiliaryCount: number
	classifiedPrimaryCount: number
	classifiedAuxiliaryCount: number
}

export function selectCatalogCounts(
	categorized: AppInfo[],
	isHidden: AppPredicate,
	isFavorite: AppPredicate,
): CatalogCounts {
	const visibleCategorizedApps: AppInfo[] = []
	const navigationCounts = new Map<string, number>()
	let favoriteCount = 0
	let hiddenCount = 0
	let auxiliaryCount = 0
	let classifiedAuxiliaryCount = 0
	for (const app of categorized) {
		const hidden = isHidden(app)
		if (hidden) hiddenCount += 1
		if (isCatalogArtifact(app)) {
			if (!hidden)
				navigationCounts.set(
					INSTALLERS_DOCS_CATEGORY,
					(navigationCounts.get(INSTALLERS_DOCS_CATEGORY) ?? 0) + 1,
				)
			continue
		}
		if (app.visibilityClass === 'auxiliary') {
			classifiedAuxiliaryCount += 1
			if (!hidden) auxiliaryCount += 1
			continue
		}
		if (hidden) continue
		visibleCategorizedApps.push(app)
		navigationCounts.set(
			app.category,
			(navigationCounts.get(app.category) ?? 0) + 1,
		)
		if (isFavorite(app)) favoriteCount += 1
	}
	return {
		visibleCategorizedApps,
		navigationCounts,
		favoriteCount,
		hiddenCount,
		auxiliaryCount,
		classifiedPrimaryCount: categorized.length - classifiedAuxiliaryCount,
		classifiedAuxiliaryCount,
	}
}

export interface CategorizedAppsState {
	apps: AppInfo[]
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	installerAppIds: string[]
	installerAppIdentities: string[]
}

export function selectCategorizedApps(state: CategorizedAppsState): AppInfo[] {
	const promotedIds = new Set(state.promotedAppIds)
	const promotedIdentities = new Set(state.promotedAppIdentities)
	const installerIds = new Set(state.installerAppIds)
	const installerIdentities = new Set(state.installerAppIdentities)
	return deduplicateVisibleApps(
		state.apps.map(app => {
			if (isCatalogArtifact(app)) {
				return app.category === INSTALLERS_DOCS_CATEGORY
					? app
					: { ...app, category: INSTALLERS_DOCS_CATEGORY }
			}
			if (
				installerIds.has(app.id) ||
				installerIdentities.has(appIdentity(app))
			)
				return {
					...app,
					artifactKind: 'installer' as const,
					category: INSTALLERS_DOCS_CATEGORY,
					userInstaller: true,
				}
			const category =
				state.categoryOverrideIdentities[appIdentity(app)] ??
				state.categoryOverrides[app.id] ??
				app.category
			const safeCategory =
				category === INSTALLERS_DOCS_CATEGORY ? app.category : category
			const promote =
				app.visibilityClass === 'auxiliary' &&
				(promotedIds.has(app.id) ||
					promotedIdentities.has(appIdentity(app)))
			if (safeCategory === app.category && !promote) return app
			const categorized = { ...app, category: safeCategory }
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
