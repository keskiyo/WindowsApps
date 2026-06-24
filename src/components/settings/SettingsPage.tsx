import {
	ExternalLink,
	FolderPlus,
	FolderSearch,
	FolderX,
	HardDrive,
	Keyboard,
	Power,
	RefreshCw,
	RotateCcw,
	Send,
	Trash2,
} from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'
import type { ScanSettings, SystemClient, SystemSettings } from '../../types'
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
	const [settings, setSettings] = useState<SystemSettings | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [saving, setSaving] = useState(false)
	const [includedPath, setIncludedPath] = useState('')
	const [excludedPath, setExcludedPath] = useState('')
	const [confirmForce, setConfirmForce] = useState(false)
	const [forcing, setForcing] = useState(false)
	const [confirmReset, setConfirmReset] = useState(false)
	const [resetting, setResetting] = useState(false)
	useEffect(() => {
		let active = true
		client
			.getSettings()
			.then(value => {
				if (active) setSettings(value)
			})
			.catch(reason => {
				if (active) setError(String(reason))
			})
		return () => {
			active = false
		}
	}, [client])
	async function toggleAutostart() {
		if (!settings || saving) return
		const enabled = !settings.autostartEnabled
		setSaving(true)
		try {
			await client.setAutostart(enabled)
			setSettings({ ...settings, autostartEnabled: enabled })
		} catch (reason) {
			setError(String(reason))
		} finally {
			setSaving(false)
		}
	}
	async function saveScanSettings(next: ScanSettings) {
		if (!settings || saving) return
		setSaving(true)
		setError(null)
		try {
			const scanSettings = await client.setScanSettings(next)
			setSettings({ ...settings, scanSettings })
		} catch (reason) {
			setError(String(reason))
		} finally {
			setSaving(false)
		}
	}
	function addPath(kind: 'includedPaths' | 'excludedPaths', value: string) {
		const trimmed = value.trim()
		if (!settings || !trimmed) return
		if (
			settings.scanSettings[kind].some(
				path => path.toLowerCase() === trimmed.toLowerCase(),
			)
		)
			return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: [...settings.scanSettings[kind], trimmed],
		})
	}
	function removePath(
		kind: 'includedPaths' | 'excludedPaths',
		value: string,
	) {
		if (!settings) return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: settings.scanSettings[kind].filter(path => path !== value),
		})
	}
	async function forceFullScan() {
		if (!onForceFullScan || forcing) return
		setForcing(true)
		setError(null)
		try {
			await onForceFullScan()
			setConfirmForce(false)
		} catch (reason) {
			setError(String(reason))
		} finally {
			setForcing(false)
		}
	}
	async function resetCatalogCache() {
		if (!onResetCatalogCache || resetting) return
		setResetting(true)
		setError(null)
		try {
			await onResetCatalogCache()
			setConfirmReset(false)
		} catch (reason) {
			setError(String(reason))
		} finally {
			setResetting(false)
		}
	}
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
			<div className='mt-6 rounded-2xl border border-white/85 bg-white/58 p-5 shadow-[7px_8px_20px_rgba(104,114,136,.11),-6px_-6px_16px_rgba(255,255,255,.72)]'>
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
				<div className='mt-5 rounded-2xl border border-white/85 bg-white/58 p-5 shadow-[7px_8px_20px_rgba(104,114,136,.11),-6px_-6px_16px_rgba(255,255,255,.72)]'>
					<div className='flex items-center gap-4'>
						<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
							<RefreshCw size={19} aria-hidden='true' />
						</span>
						<div className='min-w-0 flex-1'>
							<h2 className='font-medium'>Catalog maintenance</h2>
							<p className='mt-1 text-sm leading-6 text-slate-600'>
								Discard the incremental scan index and inspect every
								configured location again. Categories, Favorites and Hidden
								apps are preserved.
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
									className='inline-flex items-center gap-2 rounded-xl border border-red-300/70 px-4 py-2.5 text-sm font-medium text-red-700 hover:bg-red-100 disabled:opacity-50'
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
								The next scan will take longer than an incremental refresh.
							</p>
							<div className='flex gap-2'>
								<button
									type='button'
									disabled={forcing}
									onClick={() => setConfirmForce(false)}
									className='rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-slate-200/70'
								>
									Cancel
								</button>
								<button
									type='button'
									disabled={forcing}
									onClick={() => void forceFullScan()}
									className='rounded-lg bg-amber-500 px-3 py-2 text-sm font-medium text-white hover:bg-amber-400 disabled:opacity-50'
								>
									{forcing ? 'Scanning…' : 'Confirm full scan'}
								</button>
							</div>
						</div>
					)}
					{confirmReset && (
						<div
							role='dialog'
							aria-label='Confirm catalog cache reset'
							className='mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-red-300/70 bg-red-50 p-4'
						>
							<p className='text-sm text-red-800'>
								This removes the local app cache and icon cache, then scans
								every configured location again. Favorites, Hidden apps and
								categories are preserved.
							</p>
							<div className='flex gap-2'>
								<button
									type='button'
									disabled={resetting}
									onClick={() => setConfirmReset(false)}
									className='rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-slate-200/70'
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
			<div className='mt-5 overflow-hidden rounded-2xl border border-white/85 bg-white/58 shadow-[7px_8px_20px_rgba(104,114,136,.11),-6px_-6px_16px_rgba(255,255,255,.72)]'>
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
					className='flex w-full items-center gap-4 p-5 text-left hover:bg-slate-200/55 focus-visible:outline-2 focus-visible:outline-violet-500'
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

interface PathEditorProps {
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

function PathEditor(props: PathEditorProps) {
	async function browse() {
		if (props.disabled) return
		const picked = await props.onBrowse()
		if (picked) props.onAdd(picked)
	}
	return (
		<div>
			<label className='text-sm font-medium' htmlFor={props.label}>
				{props.label}
			</label>
			<div className='mt-2 flex gap-2'>
				<input
					id={props.label}
					aria-label={props.label}
					value={props.value}
					onChange={event => props.onChange(event.target.value)}
					onDoubleClick={() => void browse()}
					placeholder='D:\\Apps'
					title='Double-click to browse for a folder'
					className='h-10 min-w-0 flex-1 rounded-xl border border-slate-200 bg-white/75 px-3 text-sm text-slate-800 outline-none focus:border-violet-400/55 focus:ring-3 focus:ring-violet-500/10'
				/>
				<button
					type='button'
					aria-label={props.browseLabel}
					disabled={props.disabled}
					onClick={() => void browse()}
					className='grid size-10 shrink-0 place-items-center rounded-xl border border-slate-200 bg-white/75 text-violet-700 hover:bg-violet-50 focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-40'
				>
					<FolderSearch size={16} aria-hidden='true' />
				</button>
			</div>
			<ul className='mt-2 space-y-1'>
				{props.paths.map(path => (
					<li
						key={path}
						className='flex items-center gap-2 rounded-lg bg-slate-100/85 px-2.5 py-2 text-xs text-slate-600'
					>
						<code className='min-w-0 flex-1 truncate'>{path}</code>
						<button
							type='button'
							aria-label={`Remove ${path}`}
							onClick={() => props.onRemove(path)}
							className='grid size-7 place-items-center rounded-md hover:bg-red-100 hover:text-red-700'
						>
							<Trash2 size={14} aria-hidden='true' />
						</button>
					</li>
				))}
			</ul>
		</div>
	)
}
