import type { ScenarioRunSummary } from '../types'

export function scenarioRunSummaryMessage(
	summary: ScenarioRunSummary,
): string | null {
	const parts: string[] = []
	if (summary.launched) parts.push(`${summary.launched} launched`)
	if (summary.launchFailed) parts.push(`${summary.launchFailed} could not start`)
	if (summary.closed) parts.push(`${summary.closed} closed`)
	if (summary.notRunning) parts.push(`${summary.notRunning} already closed`)
	if (summary.blocked) parts.push(`${summary.blocked} refused to close`)
	if (summary.closeFailed) parts.push(`${summary.closeFailed} stayed open`)
	if (summary.closeUnavailable) parts.push('closing was unavailable')
	return parts.length ? `${summary.scenarioName}: ${parts.join(', ')}` : null
}
