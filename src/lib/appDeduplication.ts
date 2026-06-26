import type { AppInfo } from '../types'

/**
 * Collapses the same application discovered through multiple sources (registry, Start Menu,
 * Steam, portable) into a single visible entry. Candidates are matched by shared id,
 * lowercased path, or a launcher-stripped name "family", then merged keeping the richest
 * metadata. Looked up by those keys (O(N)) instead of scanning every survivor (O(N^2)).
 */
export function deduplicateVisibleApps(apps: AppInfo[]): AppInfo[] {
	const unique: AppInfo[] = []
	const byId = new Map<string, number>()
	const byPath = new Map<string, number>()
	const byFamily = new Map<string, number[]>()
	// Survivor keys are re-registered on merge so later apps still resolve to the merged entry.
	const register = (entry: AppInfo, position: number) => {
		byId.set(entry.id, position)
		byPath.set(entry.path.toLowerCase(), position)
		const key = stripLauncherSuffix(normalizedVisibleFamily(entry.name))
		const list = byFamily.get(key)
		if (list) {
			if (!list.includes(position)) list.push(position)
		} else byFamily.set(key, [position])
	}
	for (const app of apps) {
		const candidates = new Set<number>()
		const idHit = byId.get(app.id)
		if (idHit !== undefined) candidates.add(idHit)
		const pathHit = byPath.get(app.path.toLowerCase())
		if (pathHit !== undefined) candidates.add(pathHit)
		const familyKey = stripLauncherSuffix(normalizedVisibleFamily(app.name))
		for (const idx of byFamily.get(familyKey) ?? []) candidates.add(idx)
		const index = [...candidates]
			.sort((left, right) => left - right)
			.find(idx => isVisibleDuplicate(unique[idx], app))
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

function isVisibleDuplicate(left: AppInfo, right: AppInfo): boolean {
	if (
		left.id === right.id ||
		left.path.toLowerCase() === right.path.toLowerCase()
	)
		return true
	const leftName = normalizedVisibleFamily(left.name)
	const rightName = normalizedVisibleFamily(right.name)
	if (leftName !== rightName) {
		return (
			!publishersConflict(left, right) &&
			stripLauncherSuffix(leftName) === stripLauncherSuffix(rightName) &&
			(left.launchKind === 'shortcut' ||
				right.launchKind === 'shortcut' ||
				samePathFamily(left, right))
		)
	}
	return (
		!publishersConflict(left, right) ||
		(oneIsShortcut(left, right) && samePathFamily(left, right))
	)
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

function oneIsShortcut(left: AppInfo, right: AppInfo): boolean {
	return left.launchKind === 'shortcut' || right.launchKind === 'shortcut'
}

function publishersConflict(left: AppInfo, right: AppInfo): boolean {
	const leftPublisher = normalizedPublisher(left.publisher)
	const rightPublisher = normalizedPublisher(right.publisher)
	return Boolean(
		leftPublisher && rightPublisher && leftPublisher !== rightPublisher,
	)
}

function normalizedPublisher(value: string | null): string {
	return (
		value
			?.toLocaleLowerCase()
			.replace(
				/\b(incorporated|inc|llc|ltd|limited|corp|corporation)\b/g,
				'',
			)
			.replace(/[^a-z0-9а-яё]+/giu, '')
			.trim() ?? ''
	)
}

function samePathFamily(left: AppInfo, right: AppInfo): boolean {
	const leftPath = left.path.toLocaleLowerCase()
	const rightPath = right.path.toLocaleLowerCase()
	const leftFamily = normalizedVisibleFamily(left.name)
	const rightFamily = normalizedVisibleFamily(right.name)
	return (
		leftPath.includes(leftFamily) ||
		leftPath.includes(rightFamily) ||
		rightPath.includes(leftFamily) ||
		rightPath.includes(rightFamily)
	)
}

function normalizedVisibleFamily(name: string): string {
	let value = name.toLocaleLowerCase().split(/\s+/).join(' ').trim()
	for (const suffix of [
		' (64bit)',
		' (32bit)',
		' (64-bit)',
		' (32-bit)',
		' x64',
		' x86',
	]) {
		if (value.endsWith(suffix)) value = value.slice(0, -suffix.length)
	}
	value = stripVersionSuffix(value)
	return value
}

function stripVersionSuffix(value: string): string {
	const match = value.match(/^(.*?)[\s_-]+v?\d+(?:[._-]\d+){1,3}$/i)
	return match?.[1]?.trim() || value
}

function stripLauncherSuffix(value: string): string {
	return value.endsWith(' launcher')
		? value.slice(0, -' launcher'.length).trim()
		: value
}
