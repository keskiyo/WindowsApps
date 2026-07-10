import type { AppInfo } from '../types'

/**
 * Frontend-only safety net for stale caches. Rust owns product identity and evidence-based
 * deduplication; the UI only collapses entries that already share a canonical id or the exact
 * same normalized path.
 */
export function deduplicateVisibleApps(apps: AppInfo[]): AppInfo[] {
	const unique: AppInfo[] = []
	const byId = new Map<string, number>()
	const byPath = new Map<string, number>()

	const register = (entry: AppInfo, position: number) => {
		if (entry.id) byId.set(entry.id, position)
		byPath.set(normalizeVisiblePath(entry.path), position)
	}

	for (const app of apps) {
		const index = byId.get(app.id) ?? byPath.get(normalizeVisiblePath(app.path))
		if (index === undefined) {
			const position = unique.length
			unique.push(app)
			register(app, position)
			continue
		}

		unique[index] =
			visibleCandidateScore(app) > visibleCandidateScore(unique[index])
				? mergeVisibleApp(app, unique[index])
				: mergeVisibleApp(unique[index], app)
		register(unique[index], index)
	}

	return unique
}

function normalizeVisiblePath(value: string): string {
	return value.trim().replace(/\//g, '\\').replace(/\\+$/g, '').toLocaleLowerCase()
}

function mergeVisibleApp(primary: AppInfo, secondary: AppInfo): AppInfo {
	return {
		...primary,
		iconBase64: primary.iconBase64 ?? secondary.iconBase64,
		description: primary.description ?? secondary.description,
		version: primary.version ?? secondary.version,
		publisher: primary.publisher ?? secondary.publisher,
		installLocation: primary.installLocation ?? secondary.installLocation,
		canUninstall: primary.canUninstall || secondary.canUninstall,
	}
}

function visibleCandidateScore(app: AppInfo): number {
	if (app.sourceKind === 'steam') return 5
	if (app.launchKind === 'shortcut') return 4
	if (app.launchKind === 'executable') return 3
	if (app.launchKind === 'app_user_model_id') return 2
	return 0
}
