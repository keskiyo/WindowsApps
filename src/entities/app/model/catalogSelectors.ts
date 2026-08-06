import { deduplicateVisibleApps } from '../lib/appDeduplication'
import { appIdentity } from '../lib/appIdentity'
import {
	INSTALLERS_DOCS_CATEGORY,
	isCatalogArtifact,
} from '../lib/catalogArtifacts'
import type { AppInfo, AppView } from './app.types'
import type { AppCategory } from '../../category'

export function filterVisibleApps(
	categorized: AppInfo[],
	activeView: AppView,
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): AppInfo[] {
	if (
		activeView === 'settings' ||
		activeView === 'more' ||
		activeView === 'scenarios'
	)
		return []
	const hidden = new Set(hiddenAppIds)
	if (activeView === 'installers_docs')
		return categorized.filter(
			app => isCatalogArtifact(app) && !hidden.has(app.id),
		)
	if (activeView === 'auxiliary')
		return categorized.filter(
			app =>
				!isCatalogArtifact(app) &&
				app.visibilityClass === 'auxiliary' &&
				!hidden.has(app.id),
		)
	if (activeView === 'hidden')
		return categorized.filter(app => hidden.has(app.id))
	const visible = categorized.filter(
		app =>
			!isCatalogArtifact(app) &&
			app.visibilityClass !== 'auxiliary' &&
			!hidden.has(app.id),
	)
	if (activeView !== 'favorites') return visible
	const favorites = new Set(favoriteAppIds)
	return visible.filter(app => favorites.has(app.id))
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
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): CatalogCounts {
	const hidden = new Set(hiddenAppIds)
	const favorites = new Set(favoriteAppIds)
	const visibleCategorizedApps: AppInfo[] = []
	const navigationCounts = new Map<string, number>()
	let favoriteCount = 0
	let hiddenCount = 0
	let auxiliaryCount = 0
	let classifiedAuxiliaryCount = 0
	for (const app of categorized) {
		const isHidden = hidden.has(app.id)
		if (isHidden) hiddenCount += 1
		if (isCatalogArtifact(app)) {
			if (!isHidden)
				navigationCounts.set(
					INSTALLERS_DOCS_CATEGORY,
					(navigationCounts.get(INSTALLERS_DOCS_CATEGORY) ?? 0) + 1,
				)
			continue
		}
		if (app.visibilityClass === 'auxiliary') {
			classifiedAuxiliaryCount += 1
			if (!isHidden) auxiliaryCount += 1
			continue
		}
		if (isHidden) continue
		visibleCategorizedApps.push(app)
		navigationCounts.set(
			app.category,
			(navigationCounts.get(app.category) ?? 0) + 1,
		)
		if (favorites.has(app.id)) favoriteCount += 1
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
