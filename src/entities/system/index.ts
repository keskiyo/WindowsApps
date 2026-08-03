// Public API of the System entity. `api/systemClient.ts` is deliberately absent for the same
// reason as `entities/app`: the concrete IPC client is wired in `app/main.tsx`, not imported by
// consumers that only need the contract.
export type {
	GlobalShortcutStatus,
	ScanSettings,
	StaleCopyInfo,
	SystemClient,
	SystemSettings,
	UninstallHistoryEntry,
} from './model/system.types'
