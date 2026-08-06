import { appIdentity } from '../../entities/app'
import type { AppCategory } from '../../entities/category'
import type { AppInfo } from '../../entities/app'

export const identityOf = appIdentity

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
