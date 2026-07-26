import type { UpdateInstallPhase } from '../../../hooks/useUpdater'

export const UPDATE_STEPS: Exclude<UpdateInstallPhase, 'idle' | 'failed'>[] = [
	'downloading',
	'verifying',
	'installing',
	'restarting',
]
