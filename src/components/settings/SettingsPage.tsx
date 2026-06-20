import {
	ExternalLink,
	FolderPlus,
	FolderX,
	HardDrive,
	Keyboard,
	Power,
	Send,
	Trash2,
} from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'
import type { ScanSettings, SystemClient, SystemSettings } from '../../types'

interface Props {
	client: SystemClient
}

export function SettingsPage({ client }: Props) {
	const [settings, setSettings] = useState<SystemSettings | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [saving, setSaving] = useState(false)
	const [includedPath, setIncludedPath] = useState('')
	const [excludedPath, setExcludedPath] = useState('')
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
		if (!settings || !value.trim()) return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: [...settings.scanSettings[kind], value.trim()],
		})
	}
	function removePath(kind: 'includedPaths' | 'excludedPaths', value: string) {
		if (!settings) return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: settings.scanSettings[kind].filter(path => path !== value),
		})
	}
	return (
		<section aria-labelledby='settings-title' className='mx-auto max-w-3xl'>
			<div className='mb-8 flex items-center gap-4'>
				<img
					src='/app-icon.png'
					alt='Windows Apps logo'
					className='size-16 rounded-2xl ring-1 ring-blue-400/25'
				/>
				<div>
					<h1 id='settings-title' className='text-2xl font-semibold'>
						Settings
					</h1>
					<p className='mt-1 text-sm text-slate-400'>
						{settings
							? `Version ${settings.version}`
							: 'Loading version…'}
					</p>
				</div>
			</div>
			<div className='mt-6 rounded-2xl border border-white/8 bg-slate-900/55 p-5'>
				<div className='flex items-start gap-4'>
					<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/60 text-blue-300'>
						<HardDrive size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>Application discovery</h2>
						<p className='mt-1 text-sm leading-6 text-slate-500'>
							Scan permanent local drives and Steam libraries. Removable and network drives are ignored.
						</p>
					</div>
					<button
						type='button'
						role='switch'
						aria-label='Scan all fixed local drives'
						aria-checked={settings?.scanSettings.autoScanFixedDrives ?? false}
						disabled={!settings || saving}
						onClick={() => settings && void saveScanSettings({
							...settings.scanSettings,
							autoScanFixedDrives: !settings.scanSettings.autoScanFixedDrives,
						})}
						className={`relative h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-2 focus-visible:outline-blue-400 disabled:opacity-50 ${settings?.scanSettings.autoScanFixedDrives ? 'bg-blue-500' : 'bg-slate-700'}`}
					>
						<span className={`absolute left-1 top-1 size-5 rounded-full bg-white shadow transition-transform ${settings?.scanSettings.autoScanFixedDrives ? 'translate-x-5' : 'translate-x-0'}`} />
					</button>
				</div>

				<div className='mt-5'>
					<p className='text-xs font-semibold uppercase tracking-[.14em] text-slate-500'>Fixed local drives</p>
					<div className='mt-2 flex flex-wrap gap-2'>
						{settings?.fixedDrives.map(drive => (
							<code key={drive} className='rounded-lg border border-white/8 bg-slate-950/70 px-2.5 py-1 text-xs text-slate-300'>{drive}</code>
						))}
					</div>
				</div>

				<div className='mt-5 grid gap-5 md:grid-cols-2'>
					<PathEditor
						label='Additional scan folder'
						buttonLabel='Add scan folder'
						value={includedPath}
						paths={settings?.scanSettings.includedPaths ?? []}
						icon={<FolderPlus size={16} aria-hidden='true' />}
						disabled={!settings || saving}
						onChange={setIncludedPath}
						onAdd={() => {
							addPath('includedPaths', includedPath)
							setIncludedPath('')
						}}
						onRemove={value => removePath('includedPaths', value)}
					/>
					<PathEditor
						label='Excluded folder'
						buttonLabel='Exclude folder'
						value={excludedPath}
						paths={settings?.scanSettings.excludedPaths ?? []}
						icon={<FolderX size={16} aria-hidden='true' />}
						disabled={!settings || saving}
						onChange={setExcludedPath}
						onAdd={() => {
							addPath('excludedPaths', excludedPath)
							setExcludedPath('')
						}}
						onRemove={value => removePath('excludedPaths', value)}
					/>
				</div>
			</div>
			<div className='overflow-hidden rounded-2xl border border-white/8 bg-slate-900/55'>
				<div className='flex items-center gap-4 border-b border-white/7 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-950/60 text-blue-300'>
						<Power size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>
							Launch when Windows starts
						</h2>
						<p className='mt-1 text-sm text-slate-500'>
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
						className={`relative h-7 w-12 rounded-full transition focus-visible:outline-2 focus-visible:outline-blue-400 disabled:opacity-50 ${settings?.autostartEnabled ? 'bg-blue-500' : 'bg-slate-700'}`}
					>
						<span
							className={`absolute left-1 top-1 size-5 rounded-full bg-white shadow transition-transform ${settings?.autostartEnabled ? 'translate-x-5' : 'translate-x-0'}`}
						/>
					</button>
				</div>
				<div className='flex items-center gap-4 border-b border-white/7 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-950/60 text-blue-300'>
						<Keyboard size={19} aria-hidden='true' />
					</span>
					<div className='flex-1'>
						<h2 className='font-medium'>Global shortcut</h2>
						<p className='mt-1 text-sm text-slate-500'>
							Uses the physical Q key, independent of keyboard
							layout.
						</p>
					</div>
					<kbd className='rounded-lg border border-white/10 bg-slate-950 px-3 py-1.5 text-sm text-slate-300'>
						{settings?.shortcut.label ?? 'Win+Shift+Q'}
					</kbd>
				</div>
				<button
					type='button'
					aria-label='Open @keskiyo on Telegram'
					onClick={() => void client.openTelegram()}
					className='flex w-full items-center gap-4 p-5 text-left hover:bg-slate-800/60 focus-visible:outline-2 focus-visible:outline-blue-400'
				>
					<span className='grid size-10 place-items-center rounded-xl bg-[#229ED9]/15 text-[#5cc8f5]'>
						<Send size={19} aria-hidden='true' />
					</span>
					<span className='flex-1'>
						<span className='block font-medium'>Telegram</span>
						<span className='mt-1 block text-sm text-slate-400'>
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
				<p className='mt-4 text-sm text-amber-300'>
					{settings.shortcut.error}
				</p>
			)}
			{error && (
				<p role='alert' className='mt-4 text-sm text-red-300'>
					{error}
				</p>
			)}
		</section>
	)
}

interface PathEditorProps {
	label: string
	buttonLabel: string
	value: string
	paths: string[]
	icon: ReactNode
	disabled: boolean
	onChange(value: string): void
	onAdd(): void
	onRemove(value: string): void
}

function PathEditor(props: PathEditorProps) {
	return (
		<div>
			<label className='text-sm font-medium' htmlFor={props.label}>{props.label}</label>
			<div className='mt-2 flex gap-2'>
				<input
					id={props.label}
					aria-label={props.label}
					value={props.value}
					onChange={event => props.onChange(event.target.value)}
					placeholder='D:\\Apps'
					className='h-10 min-w-0 flex-1 rounded-xl border border-white/8 bg-slate-950/70 px-3 text-sm outline-none focus:border-blue-400/50 focus:ring-3 focus:ring-blue-500/10'
				/>
				<button
					type='button'
					aria-label={props.buttonLabel}
					disabled={props.disabled || !props.value.trim()}
					onClick={props.onAdd}
					className='grid size-10 shrink-0 place-items-center rounded-xl bg-blue-500 text-white hover:bg-blue-400 focus-visible:outline-2 focus-visible:outline-blue-300 disabled:opacity-40'
				>
					{props.icon}
				</button>
			</div>
			<ul className='mt-2 space-y-1'>
				{props.paths.map(path => (
					<li key={path} className='flex items-center gap-2 rounded-lg bg-slate-950/45 px-2.5 py-2 text-xs text-slate-400'>
						<code className='min-w-0 flex-1 truncate'>{path}</code>
						<button type='button' aria-label={`Remove ${path}`} onClick={() => props.onRemove(path)} className='grid size-7 place-items-center rounded-md hover:bg-red-500/10 hover:text-red-300'>
							<Trash2 size={14} aria-hidden='true' />
						</button>
					</li>
				))}
			</ul>
		</div>
	)
}
