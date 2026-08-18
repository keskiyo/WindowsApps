import { SettingsToggle } from '../components/SettingsToggle'
import { FolderPlus, FolderX, HardDrive } from 'lucide-react'
import { useState } from 'react'
import { PathEditor } from '../components/PathEditor'
import type { SettingsDiscoveryControlsProps } from '../../types'
import { SettingsSectionHeader } from '../components/SettingsSectionHeader'

export function SettingsDiscoveryControls({
	settings,
	saving,
	onSaveScanSettings,
	onAddPath,
	onRemovePath,
	onPickFolder,
}: SettingsDiscoveryControlsProps) {
	const [includedPath, setIncludedPath] = useState('')
	const [excludedPath, setExcludedPath] = useState('')
	const disabled = !settings || saving

	return (
		<div className="settings-surface rounded-2xl border border-white/85 bg-white/58 p-5">
			<div className="flex items-start gap-4">
				<SettingsSectionHeader
					icon={HardDrive}
					title="Application discovery"
					description="Scan permanent local drives and Steam libraries. Removable and network drives are ignored."
				/>
				<SettingsToggle
					label="Scan all fixed local drives"
					checked={
						settings?.scanSettings.autoScanFixedDrives ?? false
					}
					disabled={disabled}
					onToggle={() =>
						settings &&
						void onSaveScanSettings({
							...settings.scanSettings,
							autoScanFixedDrives:
								!settings.scanSettings.autoScanFixedDrives,
						})
					}
				/>
			</div>

			<div className="mt-5">
				<p className="text-xs font-semibold tracking-[.14em] text-slate-500 uppercase">
					Fixed local drives
				</p>
				<div className="mt-2 flex flex-wrap gap-2">
					{settings?.fixedDrives.map(drive => (
						<code
							key={drive}
							className="rounded-lg border border-slate-200 bg-white/70 px-2.5 py-1 text-xs text-slate-600"
						>
							{drive}
						</code>
					))}
				</div>
			</div>

			<div className="mt-5 grid gap-5 md:grid-cols-2">
				<PathEditor
					label="Additional scan folder"
					buttonLabel="Add scan folder"
					browseLabel="Browse for scan folder"
					value={includedPath}
					paths={settings?.scanSettings.includedPaths ?? []}
					icon={<FolderPlus size={16} aria-hidden="true" />}
					disabled={disabled}
					onChange={setIncludedPath}
					onAdd={value => {
						onAddPath('includedPaths', value)
						setIncludedPath('')
					}}
					onBrowse={onPickFolder}
					onRemove={value => onRemovePath('includedPaths', value)}
				/>
				<PathEditor
					label="Excluded folder"
					buttonLabel="Exclude folder"
					browseLabel="Browse for excluded folder"
					value={excludedPath}
					paths={settings?.scanSettings.excludedPaths ?? []}
					icon={<FolderX size={16} aria-hidden="true" />}
					disabled={disabled}
					onChange={setExcludedPath}
					onAdd={value => {
						onAddPath('excludedPaths', value)
						setExcludedPath('')
					}}
					onBrowse={onPickFolder}
					onRemove={value => onRemovePath('excludedPaths', value)}
				/>
			</div>
		</div>
	)
}
