import type { AppInfo } from '../../../../../entities/app'
import type { SignalRow } from './types'

export function buildDiagnosticsReport(apps: AppInfo[]): string {
	const records = apps.map(app =>
		[
			`Name: ${app.name}`,
			...buildSignalRows(app).map(row => `${row.label}: ${row.value}`),
			`Source: ${app.sourceKind}`,
			`Launch: ${app.launchKind}`,
			`Artifact: ${app.artifactKind ?? 'application'}`,
			`Visibility: ${app.visibilityClass ?? 'unknown'}`,
			`Reasons: ${app.categoryReasons?.join(', ') ?? 'none'}`,
		].join('\n'),
	)
	return [`Unrecognised applications: ${apps.length}`, ...records].join(
		'\n\n',
	)
}

export function buildSignalRows(app: AppInfo): SignalRow[] {
	return [
		{ label: 'Publisher', value: app.publisher },
		{ label: 'Product', value: app.productName },
		{ label: 'Description', value: app.description },
		{ label: 'Executable', value: app.originalFilename },
		{ label: 'Install location', value: app.installLocation },
	].flatMap(row =>
		row.value?.trim() ? [{ label: row.label, value: row.value }] : [],
	)
}
