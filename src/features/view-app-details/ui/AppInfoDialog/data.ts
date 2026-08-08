import {
	type AppInfo,
	categoryReasonsLabel,
	classificationReasonsLabel,
	SOURCE_LABELS,
	targetAvailabilityLabel,
} from '../../../../entities/app'
import { valueOrUnavailable } from './detailValues'

export function buildDetectionRows(app: AppInfo): [string, string][] {
	const rows: [string, string][] = [
		['Source', SOURCE_LABELS[app.sourceKind]],
		['Original filename', valueOrUnavailable(app.originalFilename)],
		[
			'Catalog state',
			app.visibilityClass === 'auxiliary'
				? 'Auxiliary tool'
				: 'Primary application',
		],
	]
	const reasons = classificationReasonsLabel(app.visibilityReasons)
	if (reasons) rows.push(['Why', reasons])
	const target = targetAvailabilityLabel(app.targetAvailability)
	if (target) rows.push(['Launch target check', target])
	const category = categoryReasonsLabel(app.categoryReasons)
	if (category) rows.push(['Why this category', category])
	return rows
}
