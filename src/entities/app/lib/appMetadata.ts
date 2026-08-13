import type {
	AppDetails,
	AppInfo,
	AppVisibilityReason,
} from '../model/app.types'
import { type CategoryDefinition, categoryLabel } from '../../category'
import { categoryReasonsLabel } from './categoryReason'

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

const TARGET_AVAILABILITY_LABELS: Record<string, string> = {
	'target.present': 'Verified on disk',
	'target.missing': 'Reported missing',
	'target.unverifiable.unmounted_volume':
		'Not checked — the drive is not mounted',
	'target.unverifiable.network': 'Not checked — network location',
	'target.unverifiable.access_denied': 'Not checked — access denied',
	'target.unverifiable.io_error': 'Not checked — the check failed',
	'target.unverifiable.relative_path':
		'Not checked — no base to resolve against',
	'target.not_applicable.aumid': 'Not a file — launched by package identity',
	'target.not_applicable.steam_uri': 'Not a file — launched through Steam',
	'target.not_applicable.protocol':
		'Not a file — launched through a protocol',
	'target.not_applicable.shell_location':
		'Not a file — a Windows shell location',
}

export function targetAvailabilityLabel(reason: string | null | undefined) {
	if (!reason) return null
	return TARGET_AVAILABILITY_LABELS[reason] ?? 'Unknown'
}

export function classificationReasonsLabel(
	reasons: AppVisibilityReason[] | undefined,
) {
	if (!reasons?.length) return null
	return reasons.map(reason => VISIBILITY_REASON_LABELS[reason]).join(', ')
}

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
	framework_package: 'Framework package',
	shell_location_shortcut: 'Shell location shortcut',
	sdk_sample: 'SDK sample',
	command_environment: 'Command environment',
	console_application: 'Console application',
	insufficient_launch_evidence: 'Insufficient launch evidence',
	unknown: 'Unknown',
} as const

export function displayVersion(version: string): string {
	return (
		version.replace(/^\s*v(?:ersion)?[\s.:]*(?=\d)/i, '').trim() || version
	)
}

export function descriptionLabel(description: string | null): string {
	return description?.trim() || 'No description available'
}

const FILE_SIZE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const

export function middleEllipsis(value: string, maxLength: number): string {
	if (value.length <= maxLength) return value
	if (maxLength <= 1) return '…'
	const leadingLength = Math.floor((maxLength - 1) / 2)
	const trailingLength = maxLength - 1 - leadingLength
	return `${value.slice(0, leadingLength)}…${value.slice(-trailingLength)}`
}

export function formatFileSize(bytes: number | null): string {
	if (bytes == null || !Number.isFinite(bytes) || bytes < 0)
		return 'Not available'
	let value = bytes
	let unitIndex = 0
	while (value >= 1024 && unitIndex < FILE_SIZE_UNITS.length - 1) {
		value /= 1024
		unitIndex += 1
	}
	const formatted =
		unitIndex === 0 ? String(value) : Number(value.toFixed(1)).toString()
	return `${formatted} ${FILE_SIZE_UNITS[unitIndex]}`
}

export function formatFileDate(unixSeconds: number | null): string {
	if (unixSeconds == null || !Number.isFinite(unixSeconds) || unixSeconds < 0)
		return 'Not available'
	const date = new Date(unixSeconds * 1000)
	if (Number.isNaN(date.getTime())) return 'Not available'
	return new Intl.DateTimeFormat('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric',
		timeZone: 'UTC',
	}).format(date)
}

function availabilityLabel(value: boolean | null): string {
	if (value == null) return 'Not available'
	return value ? 'Yes' : 'No'
}

function signatureLabel(status: AppDetails['signature']): string {
	if (status === 'verified') return 'Verified'
	if (status === 'unsigned') return 'Unsigned'
	return 'Not available'
}

function architectureLabel(architecture: AppDetails['architecture']): string {
	return architecture === 'notApplicable' ? 'Not applicable' : architecture
}

export function buildAppReport(
	app: AppInfo,
	details: AppDetails | null,
): string {
	const targetLabel =
		app.launchKind === 'app_user_model_id' ? 'Launch target' : 'Executable'
	const fileDetails = details ?? {
		fileSizeBytes: null,
		fileCreatedAt: null,
		fileModifiedAt: null,
		architecture: 'unknown' as const,
		signature: 'unavailable' as const,
		executableExists: null,
		installLocationExists: null,
	}
	return [
		'Application',
		`Name: ${app.name}`,
		`Publisher: ${app.publisher ?? 'Not available'}`,
		`Version: ${app.version ? displayVersion(app.version) : 'Not available'}`,
		`Category: ${app.category}`,
		'',
		'Installation',
		`Location: ${app.installLocation ?? 'Not available'}`,
		`${targetLabel}: ${app.path}`,
		`${targetLabel} found: ${availabilityLabel(fileDetails.executableExists)}`,
		`Install location found: ${availabilityLabel(fileDetails.installLocationExists)}`,
		'',
		'File details',
		`Size: ${formatFileSize(fileDetails.fileSizeBytes)}`,
		`Created: ${formatFileDate(fileDetails.fileCreatedAt)}`,
		`Modified: ${formatFileDate(fileDetails.fileModifiedAt)}`,
		`Architecture: ${architectureLabel(fileDetails.architecture)}`,
		`Digital signature: ${signatureLabel(fileDetails.signature)}`,
	].join('\n')
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
		| 'targetAvailability'
		| 'categoryReasons'
	>,
	categories: CategoryDefinition[],
	includeDiagnostics = false,
): [string, string][] {
	const rows: [string, string][] = [
		['Version', app.version ? displayVersion(app.version) : 'Unknown'],
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
	const targetLabel = targetAvailabilityLabel(app.targetAvailability)
	if (targetLabel) rows.push(['Launch target check', targetLabel])
	const categoryLabelReasons = categoryReasonsLabel(app.categoryReasons)
	if (categoryLabelReasons) {
		rows.push(['Why this category', categoryLabelReasons])
	}
	if (includeDiagnostics && app.visibilityScore != null) {
		rows.push(['Classification score', String(app.visibilityScore)])
	}
	return rows
}
