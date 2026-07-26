import { useSystemSettings } from '../../../hooks/useSystemSettings'
import { useUpdater } from '../../../hooks/useUpdater'
import { SettingsDiscoveryControls } from '../SettingsDiscoveryControls'
import { UninstallHistory } from '../UninstallHistory'
import { CatalogMaintenance } from './CatalogMaintenance'
import { GeneralSettings } from './GeneralSettings'
import type { SettingsPageProps } from './types'

export function SettingsPage({
	client,
	onForceFullScan,
	onResetCatalogCache,
	catalogDiagnostics,
	visibilityCounts,
	updater: sharedUpdater,
}: SettingsPageProps) {
	const {
		settings,
		error,
		saving,
		confirmForce,
		setConfirmForce,
		forcing,
		confirmReset,
		setConfirmReset,
		resetting,
		toggleAutostart,
		saveScanSettings,
		addPath,
		removePath,
		forceFullScan,
		resetCatalogCache,
	} = useSystemSettings({ client, onForceFullScan, onResetCatalogCache })
	// Hooks must run unconditionally; the local instance is a fallback for isolated
	// rendering (tests, storybook-style usage) and stays idle when App provides one.
	const localUpdater = useUpdater({ autoCheck: false })
	const updater = sharedUpdater ?? localUpdater
	return (
		<section aria-labelledby='settings-title' className='mx-auto max-w-3xl'>
			<div className='mb-8 flex items-center gap-4'>
				<img
					src='/app-icon.png'
					alt='Windows Apps logo'
					className='size-16 rounded-2xl ring-1 ring-violet-400/25'
				/>
				<div>
					<h1 id='settings-title' className='text-2xl font-semibold'>
						Settings
					</h1>
					<p className='mt-1 text-sm text-slate-600'>
						{settings
							? `Version ${settings.version}`
							: 'Loading version…'}
					</p>
				</div>
			</div>
			<SettingsDiscoveryControls
				settings={settings}
				saving={saving}
				onSaveScanSettings={saveScanSettings}
				onAddPath={addPath}
				onRemovePath={removePath}
				onPickFolder={client.pickFolder}
			/>
			<GeneralSettings
				settings={settings}
				saving={saving}
				updater={updater}
				onToggleAutostart={toggleAutostart}
				onOpenGithub={client.openGithub}
				onOpenTelegram={client.openTelegram}
			/>
			{onForceFullScan && (
				<CatalogMaintenance
					forcing={forcing}
					resetting={resetting}
					confirmForce={confirmForce}
					confirmReset={confirmReset}
					canReset={Boolean(onResetCatalogCache)}
					catalogDiagnostics={catalogDiagnostics}
					visibilityCounts={visibilityCounts}
					setConfirmForce={setConfirmForce}
					setConfirmReset={setConfirmReset}
					onForceFullScan={forceFullScan}
					onResetCatalogCache={resetCatalogCache}
				/>
			)}
			<UninstallHistory client={client} />
			{error && (
				<p role='alert' className='mt-4 text-sm text-red-700'>
					{error}
				</p>
			)}
		</section>
	)
}
