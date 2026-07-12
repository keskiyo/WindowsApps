import { categoryLabel, type AppInfo, type CategoryDefinition } from '../types'

export const SOURCE_LABELS = {
	registry: 'Registry',
	start_menu: 'Start Menu',
	start_apps: 'Windows Start Apps',
	msix: 'Microsoft Store / MSIX',
	steam: 'Steam',
	portable: 'Portable executable',
} as const

const VISIBILITY_LABELS = {
	primary: 'Primary application',
	auxiliary: 'Auxiliary tool',
	rejected: 'Rejected entry',
} as const

const VISIBILITY_REASON_LABELS = {
	start_menu_registration: 'Start Menu registration',
	windows_app_registration: 'Windows app registration',
	steam_registration: 'Steam registration',
	portable_candidate: 'Portable candidate',
	product_metadata: 'Product metadata',
	registered_product: 'Registered product',
	executable_product_match: 'Executable matches product',
	runtime_directory: 'Runtime directory',
	product_component: 'Product component',
	documentation_shortcut: 'Documentation shortcut',
	installer: 'Installer',
	maintenance_executable: 'Maintenance executable',
	insufficient_launch_evidence: 'Insufficient launch evidence',
} as const

export function descriptionLabel(description: string | null): string {
	return description?.trim() || 'No description available'
}

export function metadataRows(
	app: Pick<
		AppInfo,
		| 'version'
		| 'publisher'
		| 'productName'
		| 'originalFilename'
		| 'category'
		| 'sourceKind'
		| 'path'
		| 'installLocation'
		| 'visibilityClass'
		| 'visibilityScore'
		| 'visibilityReasons'
	>,
	categories: CategoryDefinition[],
	includeDiagnostics = false,
): [string, string][] {
	const rows: [string, string][] = [
		['Version', app.version ?? 'Unknown'],
		['Publisher', app.publisher ?? 'Unknown'],
		['Product', app.productName ?? 'Unknown'],
		['Original executable', app.originalFilename ?? 'Unknown'],
		['Category', categoryLabel(categories, app.category)],
		['Source', SOURCE_LABELS[app.sourceKind]],
		['Launch target', app.path],
		['Install location', app.installLocation ?? 'Unknown'],
	]
	if (app.visibilityClass) {
		rows.push([
			'Catalog visibility',
			VISIBILITY_LABELS[app.visibilityClass],
		])
	}
	if (app.visibilityReasons?.length) {
		rows.push([
			'Classification reasons',
			app.visibilityReasons
				.map(reason => VISIBILITY_REASON_LABELS[reason])
				.join(', '),
		])
	}
	if (includeDiagnostics && app.visibilityScore != null) {
		rows.push(['Classification score', String(app.visibilityScore)])
	}
	return rows
}
