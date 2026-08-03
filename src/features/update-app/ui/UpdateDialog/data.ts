import type { UpdateInstallPhase } from '../../model/useUpdater'

export const UPDATE_STEPS: Exclude<UpdateInstallPhase, 'idle' | 'failed'>[] = [
	'downloading',
	'verifying',
	'installing',
	'restarting',
]
