import { Box, EyeOff, ListChecks, Wrench } from 'lucide-react'
import type {
	MoreAreaPreview,
	MoreDestination,
	ScenarioPreviewControl,
} from './types'

/**
 * A stored timestamp as a day, or `null` when there is nothing trustworthy to show. Shared by both
 * preview rows: an app carries a first-seen stamp, a scenario a creation date, and neither should
 * render a date it does not actually have.
 */
export function formatPreviewDate(
	timestamp: number | null,
): { display: string; iso: string } | null {
	if (timestamp === null || !Number.isFinite(timestamp) || timestamp <= 0)
		return null
	const date = new Date(timestamp)
	if (Number.isNaN(date.getTime())) return null
	return {
		display: [date.getDate(), date.getMonth() + 1, date.getFullYear()]
			.map(value => String(value).padStart(2, '0'))
			.join('.'),
		iso: date.toISOString(),
	}
}

interface MoreCounts {
	auxiliaryCount: number
	hiddenCount: number
	installersDocsCount: number
	scenarioCount: number
	preview: MoreAreaPreview
	scenarioPreview: ScenarioPreviewControl
}

export function buildMoreDestinations({
	auxiliaryCount,
	hiddenCount,
	installersDocsCount,
	scenarioCount,
	preview,
	scenarioPreview,
}: MoreCounts): MoreDestination[] {
	return [
		{
			view: 'auxiliary',
			label: 'Auxiliary tools',
			description: 'Updaters, uninstallers and other helper executables.',
			count: auxiliaryCount,
			icon: Wrench,
			recent: { kind: 'apps', items: preview.auxiliary },
		},
		{
			view: 'scenarios',
			label: 'Scenarios',
			description: 'Start and close a set of apps in one click.',
			count: scenarioCount,
			icon: ListChecks,
			recent: {
				kind: 'scenarios',
				items: preview.scenarios,
				...scenarioPreview,
			},
		},
		{
			view: 'hidden',
			label: 'Hidden',
			description: 'Applications you removed from the catalog.',
			count: hiddenCount,
			icon: EyeOff,
			recent: { kind: 'apps', items: preview.hidden },
		},
		{
			view: 'installers_docs',
			label: 'Installers & Docs',
			description: 'Setup packages and documentation found while scanning.',
			count: installersDocsCount,
			icon: Box,
			recent: { kind: 'apps', items: preview.installersDocs },
		},
	]
}
