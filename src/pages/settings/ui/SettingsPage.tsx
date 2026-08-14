import {
	type SettingsArea,
	useSystemSettings,
} from '../../../features/edit-settings'
import { useUpdater } from '../../../features/update-app'
import { CatalogMaintenance } from './sections/CatalogMaintenance'
import { AdvancedSettings } from './sections/AdvancedSettings'
import { GeneralSettings } from './sections/GeneralSettings'
import { PreferencesBackup } from './sections/PreferencesBackup/PreferencesBackup'
import { SettingsDiscoveryControls } from './sections/SettingsDiscoveryControls'
import { UninstallHistory } from './sections/UninstallHistory'
import type { SettingsPageProps } from '../types'

export function SettingsPage({
	client,
	onExportPreferences,
	onValidatePreferencesImport,
	onImportPreferences,
	onRestorePreferencesBackup,
	onForceFullScan,
	onResetCatalogCache,
	catalogDiagnostics,
	updater: sharedUpdater,
}: SettingsPageProps) {
	const {
		settings,
		error,
		errorArea,
		saving,
		confirming,
		setConfirming,
		forcing,
		resetting,
		toggleAutostart,
		saveScanSettings,
		addPath,
		removePath,
		forceFullScan,
		resetCatalogCache,
	} = useSystemSettings({ client, onForceFullScan, onResetCatalogCache })
	const localUpdater = useUpdater({ autoCheck: false })
	const updater = sharedUpdater ?? localUpdater
	const areaError = (area: SettingsArea) =>
		error && errorArea === area ? (
			<p role="alert" className="mt-3 text-sm text-red-700">
				{error}
			</p>
		) : null
	return (
		<section aria-labelledby="settings-title" className="mx-auto max-w-3xl">
			<div className="mb-8 flex items-center gap-4">
				<img
					src="/app-icon.png"
					alt="Windows Apps logo"
					className="size-16 rounded-2xl ring-1 ring-violet-400/25"
				/>
				<div>
					<h1 id="settings-title" className="text-2xl font-semibold">
						Settings
					</h1>
					<p className="mt-1 text-sm text-slate-600">
						{settings
							? `Version ${settings.version}`
							: 'Loading version…'}
					</p>
				</div>
			</div>
			<GeneralSettings
				settings={settings}
				saving={saving}
				updater={updater}
				onToggleAutostart={toggleAutostart}
				onOpenGithub={client.openGithub}
				onOpenTelegram={client.openTelegram}
				onOpenAppsSettings={client.openAppsSettings}
			/>
			{areaError('settings')}
			{areaError('startup')}
			<AdvancedSettings>
				<SettingsDiscoveryControls
					settings={settings}
					saving={saving}
					onSaveScanSettings={saveScanSettings}
					onAddPath={addPath}
					onRemovePath={removePath}
					onPickFolder={client.pickFolder}
				/>
				{areaError('discovery')}
				{onExportPreferences &&
					onValidatePreferencesImport &&
					onImportPreferences &&
					onRestorePreferencesBackup && (
						<PreferencesBackup
							onExport={onExportPreferences}
							onSaveExport={client.savePreferencesBackup}
							onValidateImport={onValidatePreferencesImport}
							onImport={onImportPreferences}
							onRestore={onRestorePreferencesBackup}
						/>
					)}
				{onForceFullScan && (
					<CatalogMaintenance
						forcing={forcing}
						resetting={resetting}
						confirming={confirming}
						canReset={Boolean(onResetCatalogCache)}
						catalogDiagnostics={catalogDiagnostics}
						setConfirming={setConfirming}
						onForceFullScan={forceFullScan}
						onResetCatalogCache={resetCatalogCache}
					/>
				)}
				{areaError('maintenance')}
				<UninstallHistory client={client} />
			</AdvancedSettings>
		</section>
	)
}
