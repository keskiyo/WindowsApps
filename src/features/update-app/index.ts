// Public API of the update-app feature.
export { UpdateDialog } from './ui/UpdateDialog/UpdateDialog'
export { useUpdater } from './model/useUpdater'
export type {
	AvailableUpdate,
	UpdateCheckStatus,
	UpdateInstallPhase,
	UpdaterState,
} from './model/useUpdater'
