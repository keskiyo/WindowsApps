import { type AppInfo, appIdentity } from '../../app'

export interface ResolvedScenarioList {
	/** The catalog entries the stored identities still point at, in the stored order. */
	apps: AppInfo[]
	/** Identities the catalog no longer contains — an app the user uninstalled, say. */
	missing: number
}

/**
 * Resolve stored identities against the live catalog.
 *
 * A scenario outlives the catalog it was built from, so an entry can stop resolving: the app was
 * uninstalled, or a rescan has not finished yet. Those are reported as a count rather than
 * dropped silently, so the UI can say "1 unavailable" instead of quietly shrinking the list.
 */
export function resolveScenarioApps(
	identities: string[],
	apps: AppInfo[],
): ResolvedScenarioList {
	const byIdentity = new Map(apps.map(app => [appIdentity(app), app]))
	const resolved: AppInfo[] = []
	let missing = 0
	for (const identity of identities) {
		const app = byIdentity.get(identity)
		if (app) resolved.push(app)
		else missing += 1
	}
	return { apps: resolved, missing }
}
