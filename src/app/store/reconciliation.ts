import { appIdentity } from '../../entities/app'
import type { AppCategory } from '../../entities/category'
import type { AppInfo } from '../../entities/app'
import type { LegacyCanonicalPreferences } from './preferences'

export const identityOf = appIdentity

export interface CatalogMarks {
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	installerAppIds: string[]
	installerAppIdentities: string[]
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
}

function sameOrder(left: string[], right: string[]): boolean {
	return (
		left.length === right.length &&
		left.every((value, index) => value === right[index])
	)
}

function sameCategories(
	left: Record<string, AppCategory>,
	right: Record<string, AppCategory>,
): boolean {
	const keys = Object.keys(left)
	return (
		keys.length === Object.keys(right).length &&
		keys.every(key => left[key] === right[key])
	)
}

function marksEqual(left: CatalogMarks, right: CatalogMarks): boolean {
	return (
		sameOrder(left.favoriteAppIds, right.favoriteAppIds) &&
		sameOrder(left.favoriteAppIdentities, right.favoriteAppIdentities) &&
		sameOrder(left.hiddenAppIds, right.hiddenAppIds) &&
		sameOrder(left.hiddenAppIdentities, right.hiddenAppIdentities) &&
		sameOrder(left.promotedAppIds, right.promotedAppIds) &&
		sameOrder(left.promotedAppIdentities, right.promotedAppIdentities) &&
		sameOrder(left.installerAppIds, right.installerAppIds) &&
		sameOrder(left.installerAppIdentities, right.installerAppIdentities) &&
		sameCategories(left.categoryOverrides, right.categoryOverrides) &&
		sameCategories(
			left.categoryOverrideIdentities,
			right.categoryOverrideIdentities,
		) &&
		sameOrder(
			left.legacyCanonicalPreferences.favorite,
			right.legacyCanonicalPreferences.favorite,
		) &&
		sameOrder(
			left.legacyCanonicalPreferences.hidden,
			right.legacyCanonicalPreferences.hidden,
		) &&
		sameOrder(
			left.legacyCanonicalPreferences.promoted,
			right.legacyCanonicalPreferences.promoted,
		) &&
		sameOrder(
			left.legacyCanonicalPreferences.installer,
			right.legacyCanonicalPreferences.installer,
		) &&
		sameCategories(
			left.legacyCanonicalPreferences.categoryOverrides,
			right.legacyCanonicalPreferences.categoryOverrides,
		)
	)
}

export function reconcileMarks(
	current: CatalogMarks,
	apps: AppInfo[],
): CatalogMarks | null {
	if (apps.length === 0) return null
	const legacy = current.legacyCanonicalPreferences
	const favorites = reconcileSelection(
		apps,
		current.favoriteAppIds,
		current.favoriteAppIdentities,
		legacy.favorite,
	)
	const hidden = reconcileSelection(
		apps,
		current.hiddenAppIds,
		current.hiddenAppIdentities,
		legacy.hidden,
	)
	const promoted = reconcileSelection(
		apps,
		current.promotedAppIds,
		current.promotedAppIdentities,
		legacy.promoted,
	)
	const installers = reconcileSelection(
		apps,
		current.installerAppIds,
		current.installerAppIdentities,
		legacy.installer,
	)
	const overrides = reconcileOverrides(
		apps,
		current.categoryOverrides,
		current.categoryOverrideIdentities,
		legacy.categoryOverrides,
	)
	const next: CatalogMarks = {
		favoriteAppIds: favorites.ids,
		favoriteAppIdentities: favorites.identities,
		hiddenAppIds: hidden.ids,
		hiddenAppIdentities: hidden.identities,
		promotedAppIds: promoted.ids,
		promotedAppIdentities: promoted.identities,
		installerAppIds: installers.ids,
		installerAppIdentities: installers.identities,
		categoryOverrides: overrides.overrides,
		categoryOverrideIdentities: overrides.overrideIdentities,
		legacyCanonicalPreferences: {
			favorite: favorites.unresolvedLegacy,
			hidden: hidden.unresolvedLegacy,
			promoted: promoted.unresolvedLegacy,
			installer: installers.unresolvedLegacy,
			categoryOverrides: overrides.unresolvedLegacy,
		},
	}
	return marksEqual(current, next) ? null : next
}

export function reconcileFirstSeen(
	apps: AppInfo[],
	previous: Record<string, number>,
	now: number,
): Record<string, number> {
	const next: Record<string, number> = {}
	let changed = false
	for (const app of apps) {
		const identity = identityOf(app)
		if (identity in next) continue
		const seenAt = previous[identity]
		if (seenAt === undefined) changed = true
		next[identity] = seenAt ?? now
	}
	return changed || Object.keys(next).length !== Object.keys(previous).length
		? next
		: previous
}

export function addUnique(list: string[], value: string): string[] {
	return list.includes(value) ? list : [...list, value]
}

export function mergeIcon(
	previous: AppInfo | undefined,
	next: AppInfo,
): AppInfo {
	return previous?.iconBase64 && !next.iconBase64
		? { ...next, iconBase64: previous.iconBase64 }
		: next
}

function groupByCanonicalIdentity(apps: AppInfo[]): Map<string, AppInfo[]> {
	const groups = new Map<string, AppInfo[]>()
	for (const app of apps) {
		if (!app.canonicalIdentity) continue
		const group = groups.get(app.canonicalIdentity) ?? []
		group.push(app)
		groups.set(app.canonicalIdentity, group)
	}
	return groups
}

export function reconcileSelection(
	apps: AppInfo[],
	ids: string[],
	identities: string[],
	legacyCanonicalIdentities: string[],
): { ids: string[]; identities: string[]; unresolvedLegacy: string[] } {
	const byId = new Map(apps.map(app => [app.id, app]))
	const mergedIdentities = new Set(identities)
	const unresolvedLegacy = new Set(legacyCanonicalIdentities)
	for (const legacyId of ids) {
		const app = byId.get(legacyId)
		if (!app) continue
		mergedIdentities.add(identityOf(app))
		if (app.canonicalIdentity)
			unresolvedLegacy.delete(app.canonicalIdentity)
	}
	const byCanonicalIdentity = groupByCanonicalIdentity(apps)
	for (const legacyIdentity of unresolvedLegacy) {
		const matches = byCanonicalIdentity.get(legacyIdentity)
		if (matches?.length !== 1) continue
		mergedIdentities.add(identityOf(matches[0]))
		unresolvedLegacy.delete(legacyIdentity)
	}
	const currentIds = apps
		.filter(app => mergedIdentities.has(identityOf(app)))
		.map(app => app.id)
	return {
		ids: currentIds,
		identities: [...mergedIdentities],
		unresolvedLegacy: [...unresolvedLegacy],
	}
}

export function reconcileOverrides(
	apps: AppInfo[],
	idOverrides: Record<string, AppCategory>,
	identityOverrides: Record<string, AppCategory>,
	legacyCanonicalOverrides: Record<string, AppCategory>,
): {
	overrides: Record<string, AppCategory>
	overrideIdentities: Record<string, AppCategory>
	unresolvedLegacy: Record<string, AppCategory>
} {
	const byId = new Map(apps.map(app => [app.id, app]))
	const mergedIdentities: Record<string, AppCategory> = {
		...identityOverrides,
	}
	const unresolvedLegacy = { ...legacyCanonicalOverrides }
	for (const [legacyId, category] of Object.entries(idOverrides)) {
		const app = byId.get(legacyId)
		if (app && !(identityOf(app) in mergedIdentities))
			mergedIdentities[identityOf(app)] = category
		if (app?.canonicalIdentity)
			delete unresolvedLegacy[app.canonicalIdentity]
	}
	const byCanonicalIdentity = groupByCanonicalIdentity(apps)
	for (const [legacyIdentity, category] of Object.entries(unresolvedLegacy)) {
		const matches = byCanonicalIdentity.get(legacyIdentity)
		if (matches?.length !== 1) continue
		if (!(identityOf(matches[0]) in mergedIdentities))
			mergedIdentities[identityOf(matches[0])] = category
		delete unresolvedLegacy[legacyIdentity]
	}
	const overrides: Record<string, AppCategory> = {}
	for (const app of apps) {
		const category = mergedIdentities[identityOf(app)]
		if (category) overrides[app.id] = category
	}
	return {
		overrides,
		overrideIdentities: mergedIdentities,
		unresolvedLegacy,
	}
}
