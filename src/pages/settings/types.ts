import type { MaintenanceConfirmation } from '../../features/edit-settings'
import type { UpdaterState } from '../../features/update-app'
import type { CatalogDiagnostics } from '../../entities/app'
import type { SystemClient, SystemSettings } from '../../entities/system'

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
	onOpenAppsSettings: SystemClient['openAppsSettings']
}

export interface CatalogMaintenanceProps {
	forcing: boolean
	resetting: boolean
	/** Which action is waiting for an answer; only one ever is. */
	confirming: MaintenanceConfirmation
	canReset: boolean
	catalogDiagnostics?: CatalogDiagnostics | null
	visibilityCounts?: { primary: number; auxiliary: number }
	setConfirming(value: MaintenanceConfirmation): void
	onForceFullScan(): Promise<void>
	onResetCatalogCache(): Promise<void>
}

export interface ScanDiagnosticsProps {
	diagnostics: CatalogDiagnostics
}
