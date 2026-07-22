import { FolderPlus, FolderX, HardDrive } from 'lucide-react'
import { useState } from 'react'
import type { ScanSettings, SystemSettings } from '../../types'
import { PathEditor } from './PathEditor'

type ScanPathKind = 'includedPaths' | 'excludedPaths'

interface Props {
	settings: SystemSettings | null
	saving: boolean
	onSaveScanSettings(settings: ScanSettings): Promise<void>
	onAddPath(kind: ScanPathKind, value: string): void
	onRemovePath(kind: ScanPathKind, value: string): void
	onPickFolder(): Promise<string | null>
}

export function SettingsDiscoveryControls({
	settings,
	saving,
	onSaveScanSettings,
	onAddPath,
	onRemovePath,
	onPickFolder,
}: Props) {
	const [includedPath, setIncludedPath] = useState('')
	const [excludedPath, setExcludedPath] = useState('')
	const disabled = !settings || saving

	return (
		<div className='settings-surface mt-6 rounded-2xl border border-white/85 bg-white/58 p-5'>
			<div className='flex items-start gap-4'>
				<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
					<HardDrive size={19} aria-hidden='true' />
				</span>
				<div className='min-w-0 flex-1'>
					<h2 className='font-medium'>Application discovery</h2>
					<p className='mt-1 text-sm leading-6 text-slate-600'>
						Scan permanent local drives and Steam libraries. Removable and
						network drives are ignored.
					</p>
				</div>
				<button
					type='button'
					role='switch'
					aria-label='Scan all fixed local drives'
					aria-checked={settings?.scanSettings.autoScanFixedDrives ?? false}
					disabled={disabled}
					onClick={() =>
						settings &&
						void onSaveScanSettings({
							...settings.scanSettings,
							autoScanFixedDrives:
								!settings.scanSettings.autoScanFixedDrives,
						})
					}
					className={`relative h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-50 ${settings?.scanSettings.autoScanFixedDrives ? 'bg-violet-600' : 'bg-slate-300'}`}
				>
					<span
						className={`absolute left-1 top-1 size-5 rounded-full bg-slate-50 shadow transition-transform ${settings?.scanSettings.autoScanFixedDrives ? 'translate-x-5' : 'translate-x-0'}`}
					/>
				</button>
			</div>

			<div className='mt-5'>
				<p className='text-xs font-semibold uppercase tracking-[.14em] text-slate-500'>
					Fixed local drives
				</p>
				<div className='mt-2 flex flex-wrap gap-2'>
					{settings?.fixedDrives.map(drive => (
						<code
							key={drive}
							className='rounded-lg border border-slate-200 bg-white/70 px-2.5 py-1 text-xs text-slate-600'
						>
							{drive}
						</code>
					))}
				</div>
			</div>

			<div className='mt-5 grid gap-5 md:grid-cols-2'>
				<PathEditor
					label='Additional scan folder'
					buttonLabel='Add scan folder'
					browseLabel='Browse for scan folder'
					value={includedPath}
					paths={settings?.scanSettings.includedPaths ?? []}
					icon={<FolderPlus size={16} aria-hidden='true' />}
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
					label='Excluded folder'
					buttonLabel='Exclude folder'
					browseLabel='Browse for excluded folder'
					value={excludedPath}
					paths={settings?.scanSettings.excludedPaths ?? []}
					icon={<FolderX size={16} aria-hidden='true' />}
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
