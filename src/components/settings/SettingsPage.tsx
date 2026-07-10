import {
	ExternalLink,
	FolderPlus,
	FolderX,
	HardDrive,
	Keyboard,
	Power,
	RefreshCw,
	RotateCcw,
	Send,
} from 'lucide-react'
import { useState } from 'react'
import { useSystemSettings } from '../../hooks/useSystemSettings'
import type { SystemClient } from '../../types'
import { PathEditor } from './PathEditor'
import { UninstallHistory } from './UninstallHistory'

interface Props {
	client: SystemClient
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
}

export function SettingsPage({
	client,
	onForceFullScan,
	onResetCatalogCache,
}: Props) {
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
	const [includedPath, setIncludedPath] = useState('')
	const [excludedPath, setExcludedPath] = useState('')
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
			<div className='settings-surface mt-6 rounded-2xl border border-white/85 bg-white/58 p-5'>
				<div className='flex items-start gap-4'>
					<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<HardDrive size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>Application discovery</h2>
						<p className='mt-1 text-sm leading-6 text-slate-600'>
							Scan permanent local drives and Steam libraries.
							Removable and network drives are ignored.
						</p>
					</div>
					<button
						type='button'
						role='switch'
						aria-label='Scan all fixed local drives'
						aria-checked={
							settings?.scanSettings.autoScanFixedDrives ?? false
						}
						disabled={!settings || saving}
						onClick={() =>
							settings &&
							void saveScanSettings({
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
						disabled={!settings || saving}
						onChange={setIncludedPath}
						onAdd={value => {
							addPath('includedPaths', value)
							setIncludedPath('')
						}}
						onBrowse={() => client.pickFolder()}
						onRemove={value => removePath('includedPaths', value)}
					/>
					<PathEditor
						label='Excluded folder'
						buttonLabel='Exclude folder'
						browseLabel='Browse for excluded folder'
						value={excludedPath}
						paths={settings?.scanSettings.excludedPaths ?? []}
						icon={<FolderX size={16} aria-hidden='true' />}
						disabled={!settings || saving}
						onChange={setExcludedPath}
						onAdd={value => {
							addPath('excludedPaths', value)
							setExcludedPath('')
						}}
						onBrowse={() => client.pickFolder()}
						onRemove={value => removePath('excludedPaths', value)}
					/>
				</div>
			</div>
			{onForceFullScan && (
				<div className='settings-surface mt-5 rounded-2xl border border-white/85 bg-white/58 p-5'>
					<div className='flex items-center gap-4'>
						<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
							<RefreshCw size={19} aria-hidden='true' />
						</span>
						<div className='min-w-0 flex-1'>
							<h2 className='font-medium'>Catalog maintenance</h2>
							<p className='mt-1 text-sm leading-6 text-slate-600'>
								Discard the incremental scan index and inspect
								every configured location again. Categories,
								Favorites and Hidden apps are preserved.
							</p>
						</div>
						<div className='flex shrink-0 flex-wrap gap-2'>
							<button
								type='button'
								disabled={forcing || resetting}
								onClick={() => setConfirmForce(true)}
								className='rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white shadow-[0_8px_18px_rgba(104,69,216,.18)] hover:bg-violet-500 disabled:opacity-50'
							>
								Force full scan
							</button>
							{onResetCatalogCache && (
								<button
									type='button'
									disabled={forcing || resetting}
									onClick={() => setConfirmReset(true)}
									className='danger-button inline-flex items-center gap-2 rounded-xl border border-red-300/70 px-4 py-2.5 text-sm font-medium text-red-700 hover:bg-red-100 disabled:opacity-50'
								>
									<RotateCcw size={16} aria-hidden='true' />
									Reset catalog cache
								</button>
							)}
						</div>
					</div>
					{confirmForce && (
						<div
							role='dialog'
							aria-label='Confirm full scan'
							className='mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-300/70 bg-amber-50 p-4'
						>
							<p className='text-sm text-amber-800'>
								The next scan will take longer than an
								incremental refresh.
							</p>
							<div className='flex gap-2'>
								<button
									type='button'
									disabled={forcing}
									onClick={() => setConfirmForce(false)}
									className='rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-violet-100/70'
								>
									Cancel
								</button>
								<button
									type='button'
									disabled={forcing}
									onClick={() => void forceFullScan()}
									className='rounded-lg bg-amber-500 px-3 py-2 text-sm font-medium text-white hover:bg-amber-400 disabled:opacity-50'
								>
									{forcing
										? 'Scanning…'
										: 'Confirm full scan'}
								</button>
							</div>
						</div>
					)}
					{confirmReset && (
						<div
							role='dialog'
							aria-label='Confirm catalog cache reset'
							className='danger-panel mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-red-300/70 bg-red-50 p-4'
						>
							<p className='text-sm text-red-800'>
								This removes the local app cache and icon cache,
								then scans every configured location again.
								Favorites, Hidden apps and categories are
								preserved.
							</p>
							<div className='flex gap-2'>
								<button
									type='button'
									disabled={resetting}
									onClick={() => setConfirmReset(false)}
									className='rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-violet-100/70'
								>
									Cancel
								</button>
								<button
									type='button'
									disabled={resetting}
									onClick={() => void resetCatalogCache()}
									className='rounded-lg bg-red-500 px-3 py-2 text-sm font-medium text-white hover:bg-red-400 disabled:opacity-50'
								>
									{resetting ? 'Resetting…' : 'Confirm reset'}
								</button>
							</div>
						</div>
					)}
				</div>
			)}
			<div className='settings-surface mt-5 overflow-hidden rounded-2xl border border-white/85 bg-white/58'>
				<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<Power size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>
							Launch when Windows starts
						</h2>
						<p className='mt-1 text-sm text-slate-600'>
							Open Windows Apps automatically after you sign in.
						</p>
					</div>
					<button
						type='button'
						role='switch'
						aria-label='Launch when Windows starts'
						aria-checked={settings?.autostartEnabled ?? false}
						disabled={!settings || saving}
						onClick={() => void toggleAutostart()}
						className={`relative h-7 w-12 rounded-full transition focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-50 ${settings?.autostartEnabled ? 'bg-violet-600' : 'bg-slate-300'}`}
					>
						<span
							className={`absolute left-1 top-1 size-5 rounded-full bg-slate-50 shadow transition-transform ${settings?.autostartEnabled ? 'translate-x-5' : 'translate-x-0'}`}
						/>
					</button>
				</div>
				<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<Keyboard size={19} aria-hidden='true' />
					</span>
					<div className='flex-1'>
						<h2 className='font-medium'>Global shortcut</h2>
						<p className='mt-1 text-sm text-slate-600'>
							Uses the physical Q key, independent of keyboard
							layout.
						</p>
					</div>
					<kbd className='rounded-lg border border-slate-300 bg-slate-100 px-3 py-1.5 text-sm text-slate-700'>
						{settings?.shortcut.label ?? 'Win+Shift+Q'}
					</kbd>
				</div>
				<button
					type='button'
					aria-label='Open @keskiyo on Telegram'
					onClick={() => void client.openTelegram()}
					className='flex w-full items-center gap-4 p-5 text-left hover:bg-violet-100/55 focus-visible:outline-2 focus-visible:outline-violet-500'
				>
					<span className='grid size-10 place-items-center rounded-xl bg-[#229ED9]/15 text-[#5cc8f5]'>
						<Send size={19} aria-hidden='true' />
					</span>
					<span className='flex-1'>
						<span className='block font-medium'>Telegram</span>
						<span className='mt-1 block text-sm text-slate-600'>
							@keskiyo
						</span>
					</span>
					<ExternalLink
						size={17}
						className='text-slate-500'
						aria-hidden='true'
					/>
				</button>
			</div>
			{settings?.shortcut.error && (
				<p className='mt-4 text-sm text-amber-700'>
					{settings.shortcut.error}
				</p>
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
