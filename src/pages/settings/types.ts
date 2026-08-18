import type { ReactNode } from 'react'
import type { MaintenanceConfirmation } from '../../features/edit-settings'
import type { UpdaterState } from '../../features/update-app'
import type {
	AppInfo,
	CatalogDiagnostics,
	SourceHealth,
	TargetAvailabilityDiff,
} from '../../entities/app'
import type { AppCategory, CategoryDefinition } from '../../entities/category'
import type {
	ScanSettings,
	SystemClient,
	SystemSettings,
} from '../../entities/system'

export type ScanPathKind = 'includedPaths' | 'excludedPaths'

export type SettingsSectionIcon = (props: {
	size?: number
	'aria-hidden'?: boolean | 'true' | 'false'
}) => ReactNode

export interface SettingsSectionHeaderProps {
	icon: SettingsSectionIcon
	title: string
	description: ReactNode
}

export interface SettingsPageProps {
	client: SystemClient
	onExportPreferences?: () => string
	onValidatePreferencesImport?: (
		source: string,
	) => { ok: true } | { ok: false; error: string }
	onImportPreferences?: (
		source: string,
	) => { ok: true } | { ok: false; error: string }
	onRestorePreferencesBackup?: () =>
		{ ok: true } | { ok: false; error: string }
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
	catalogDiagnostics?: CatalogDiagnostics | null
	unclassifiedApps?: AppInfo[]
	categories?: CategoryDefinition[]
	categoryOrder?: AppCategory[]
	onMoveApp?(appId: string, category: AppCategory): void
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
	confirming: MaintenanceConfirmation
	canReset: boolean
	catalogDiagnostics?: CatalogDiagnostics | null
	setConfirming(value: MaintenanceConfirmation): void
	onForceFullScan(): Promise<void>
	onResetCatalogCache(): Promise<void>
}

export interface ScanDiagnosticsProps {
	diagnostics: CatalogDiagnostics
}

export interface SourceHealthTableProps {
	sources: SourceHealth[]
}

export interface TargetAvailabilityPanelProps {
	diff?: TargetAvailabilityDiff
}

export interface SettingsDiscoveryControlsProps {
	settings: SystemSettings | null
	saving: boolean
	onSaveScanSettings(settings: ScanSettings): Promise<void>
	onAddPath(kind: ScanPathKind, value: string): void
	onRemovePath(kind: ScanPathKind, value: string): void
	onPickFolder(): Promise<string | null>
}

export interface SettingsUpdateControlsProps {
	updater: UpdaterState
	onOpenGithub(): Promise<void>
	onOpenTelegram(): Promise<void>
}

export interface PathEditorProps {
	label: string
	buttonLabel: string
	browseLabel: string
	value: string
	paths: string[]
	icon: ReactNode
	disabled: boolean
	onChange(value: string): void
	onAdd(value: string): void
	onBrowse(): Promise<string | null>
	onRemove(value: string): void
}

export interface SettingsToggleProps {
	label: string
	checked: boolean
	disabled?: boolean
	onToggle(): void
}

export interface UninstallHistoryProps {
	client: SystemClient
}
