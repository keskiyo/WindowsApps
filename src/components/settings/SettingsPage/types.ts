import type { UpdaterState } from '../../../hooks/useUpdater'
import type {
	CatalogDiagnostics,
	SystemClient,
	SystemSettings,
} from '../../../types'

export interface SettingsPageProps {
	client: SystemClient
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
	catalogDiagnostics?: CatalogDiagnostics | null
	visibilityCounts?: { primary: number; auxiliary: number }
	/**
	 * Shared updater state from App. Without it, "Check updates" would run on a second
	 * updater instance while the update dialog listens to App's instance — a manual check
	 * would then never reopen the dialog after the user dismissed it.
	 */
	updater?: UpdaterState
}

export interface GeneralSettingsProps {
	settings: SystemSettings | null
	saving: boolean
	updater: UpdaterState
	onToggleAutostart(): Promise<void>
	onOpenGithub: SystemClient['openGithub']
	onOpenTelegram: SystemClient['openTelegram']
}

export interface CatalogMaintenanceProps {
	forcing: boolean
	resetting: boolean
	confirmForce: boolean
	confirmReset: boolean
	canReset: boolean
	catalogDiagnostics?: CatalogDiagnostics | null
	visibilityCounts?: { primary: number; auxiliary: number }
	setConfirmForce(value: boolean): void
	setConfirmReset(value: boolean): void
	onForceFullScan(): Promise<void>
	onResetCatalogCache(): Promise<void>
}

export interface ScanDiagnosticsProps {
	diagnostics: CatalogDiagnostics
}
