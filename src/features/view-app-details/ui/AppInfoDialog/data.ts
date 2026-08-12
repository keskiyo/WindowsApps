import {
	type AppInfo,
	categoryReasonsLabel,
	classificationReasonsLabel,
	SOURCE_LABELS,
	targetAvailabilityLabel,
} from '../../../../entities/app'

export function buildDetectionRows(app: AppInfo): [string, string][] {
	const rows: [string, string][] = [['Source', SOURCE_LABELS[app.sourceKind]]]
	if (app.originalFilename) {
		rows.push(['Original filename', app.originalFilename])
	}
	rows.push([
		'Catalog visibility',
		app.visibilityClass === 'auxiliary'
			? 'Shown in Auxiliary tools'
			: 'Shown in the main catalog',
	])
	const reasons = visibilityExplanation(app)
	if (reasons) rows.push(['Why shown here', reasons])
	const target = targetAvailabilityLabel(app.targetAvailability)
	if (target) rows.push(['Launch target check', target])
	const category = categoryExplanation(app)
	if (category) rows.push(['Category', category])
	return rows
}

function visibilityExplanation(app: AppInfo): string | null {
	const reasons = app.visibilityReasons ?? []
	if (
		reasons.includes('portable_candidate') &&
		reasons.includes('insufficient_launch_evidence')
	) {
		return 'This executable has no reliable product metadata yet.'
	}
	return classificationReasonsLabel(reasons)
}

function categoryExplanation(app: AppInfo): string | null {
	if (app.categoryReasons?.includes('default=no-signal')) {
		return 'Other — no category evidence'
	}
	return categoryReasonsLabel(app.categoryReasons)
}
