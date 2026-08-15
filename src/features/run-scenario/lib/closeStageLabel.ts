import type { CloseProgress } from '../../../entities/app'

export function closeStageLabel(progress: CloseProgress): string {
	if (progress.stage === 'asking')
		return progress.running === 1
			? 'asking 1 app to close'
			: `asking ${progress.running} apps to close`
	if (progress.stage === 'waiting')
		return `waiting ${progress.secondsLeft} s before force-closing`
	return 'force-closing what stayed open'
}
