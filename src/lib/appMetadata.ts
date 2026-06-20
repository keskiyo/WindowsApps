import { categoryLabel, type AppInfo, type CategoryDefinition } from '../types'

export const SOURCE_LABELS = {
	registry: 'Registry',
	start_menu: 'Start Menu',
	start_apps: 'Windows Start Apps',
	msix: 'Microsoft Store / MSIX',
} as const

export function descriptionLabel(description: string | null): string {
	return description?.trim() || 'No description available'
}

export function metadataRows(
	app: Pick<
		AppInfo,
		| 'version'
		| 'publisher'
		| 'category'
		| 'sourceKind'
		| 'path'
		| 'installLocation'
	>,
	categories: CategoryDefinition[],
): [string, string][] {
	return [
		['Version', app.version ?? 'Unknown'],
		['Publisher', app.publisher ?? 'Unknown'],
		['Category', categoryLabel(categories, app.category)],
		['Source', SOURCE_LABELS[app.sourceKind]],
		['Launch target', app.path],
		['Install location', app.installLocation ?? 'Unknown'],
	]
}
