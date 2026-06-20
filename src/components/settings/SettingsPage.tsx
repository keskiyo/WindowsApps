import { ExternalLink, Keyboard, Power, Send } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { SystemClient, SystemSettings } from '../../types'

interface Props {
	client: SystemClient
}

export function SettingsPage({ client }: Props) {
	const [settings, setSettings] = useState<SystemSettings | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [saving, setSaving] = useState(false)
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
