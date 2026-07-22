import {
	Image,
	Keyboard,
	Power,
	RefreshCw,
	RotateCcw,
	Wrench,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useSystemSettings } from '../../hooks/useSystemSettings'
import {
	useUpdater,
	type UpdaterState,
} from '../../hooks/useUpdater'
import type { CatalogDiagnostics, SystemClient } from '../../types'
import { UninstallHistory } from './UninstallHistory'
import { SettingsDiscoveryControls } from './SettingsDiscoveryControls'
import { SettingsUpdateControls } from './SettingsUpdateControls'

interface Props {
	client: SystemClient
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
	catalogDiagnostics?: CatalogDiagnostics | null
	onClearIconCache?: () => Promise<void>
	onRepairMissingIcons?: () => Promise<void>
	visibilityCounts?: { primary: number; auxiliary: number }
	/**
	 * Shared updater state from App. Without it, "Check updates" would run on a second
	 * updater instance while the update dialog listens to App's instance — a manual check
	 * would then never reopen the dialog after the user dismissed it.
	 */
	updater?: UpdaterState
}

export function SettingsPage({
	client,
	onForceFullScan,
	onResetCatalogCache,
	catalogDiagnostics,
	onClearIconCache,
	onRepairMissingIcons,
	visibilityCounts,
	updater: sharedUpdater,
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
	const [iconAction, setIconAction] = useState<'clear' | 'repair' | null>(null)
	const [iconMessage, setIconMessage] = useState<string | null>(null)
	const forceTriggerRef = useRef<HTMLButtonElement>(null)
	const resetTriggerRef = useRef<HTMLButtonElement>(null)
	const previousConfirmForce = useRef(false)
	const previousConfirmReset = useRef(false)
	useEffect(() => {
		if (previousConfirmForce.current && !confirmForce)
			forceTriggerRef.current?.focus()
		previousConfirmForce.current = confirmForce
	}, [confirmForce])
	useEffect(() => {
		if (previousConfirmReset.current && !confirmReset)
			resetTriggerRef.current?.focus()
		previousConfirmReset.current = confirmReset
	}, [confirmReset])
	// Hooks must run unconditionally; the local instance is a fallback for isolated
	// rendering (tests, storybook-style usage) and stays idle when App provides one.
	const localUpdater = useUpdater({ autoCheck: false })
	const updater = sharedUpdater ?? localUpdater
	async function runIconAction(
		action: 'clear' | 'repair',
		operation: (() => Promise<void>) | undefined,
	) {
		if (!operation || iconAction) return
		setIconAction(action)
		setIconMessage(null)
		try {
			await operation()
			setIconMessage(
				action === 'clear'
					? 'Icon cache cleared and icon recovery started.'
					: 'Missing icon recovery started.',
			)
		} catch {
			setIconMessage('Icon maintenance could not be completed.')
		} finally {
			setIconAction(null)
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
			<SettingsDiscoveryControls
				settings={settings}
				saving={saving}
				onSaveScanSettings={saveScanSettings}
				onAddPath={addPath}
				onRemovePath={removePath}
				onPickFolder={client.pickFolder}
			/>
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
				<SettingsUpdateControls
					updater={updater}
					onOpenGithub={client.openGithub}
					onOpenTelegram={client.openTelegram}
				/>
			</div>
			{settings?.shortcut.error && (
				<p className='mt-4 text-sm text-amber-700'>
					{settings.shortcut.error}
				</p>
			)}
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
								ref={forceTriggerRef}
								type='button'
								disabled={forcing || resetting}
								onClick={() => setConfirmForce(true)}
								className='rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white shadow-[0_8px_18px_rgba(104,69,216,.18)] hover:bg-violet-500 disabled:opacity-50'
							>
								Force full scan
							</button>
							{onResetCatalogCache && (
								<button
									ref={resetTriggerRef}
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
							className='mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-violet-400/35 bg-violet-500/8 p-4 shadow-inner shadow-violet-950/10'
						>
							<p className='text-sm leading-6 text-slate-700'>
								The next scan will take longer than an
								incremental refresh.
							</p>
							<div className='ml-auto flex gap-2'>
								<button
									type='button'
									disabled={forcing}
									onClick={() => setConfirmForce(false)}
									className='rounded-lg border border-slate-300/80 bg-white/60 px-3 py-2 text-sm text-slate-700 transition-colors hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-400 disabled:opacity-50'
								>
									Cancel
								</button>
								<button
									type='button'
									disabled={forcing}
									onClick={() => void forceFullScan()}
									className='rounded-lg bg-violet-600 px-3 py-2 text-sm font-medium text-white shadow-[0_8px_18px_rgba(124,58,237,.22)] transition-colors hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-50'
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
					{visibilityCounts && (
						<div className='mt-5 flex flex-wrap gap-x-6 gap-y-2 border-t border-slate-200/80 pt-4 text-sm'>
							<span className='text-slate-600'>Primary applications</span>
							<span className='font-medium text-slate-800'>{visibilityCounts.primary}</span>
							<span className='text-slate-600'>Auxiliary tools</span>
							<span className='font-medium text-slate-800'>{visibilityCounts.auxiliary}</span>
						</div>
					)}
					{catalogDiagnostics && (
						<div className='mt-5 border-t border-slate-200/80 pt-4'>
							<div className='flex items-center gap-2 text-sm font-medium text-slate-800'>
								<Wrench size={16} aria-hidden='true' />
								Last scan diagnostics
							</div>
							<div className='mt-3 grid grid-cols-2 gap-x-5 gap-y-2 text-sm sm:grid-cols-4'>
								<span className='text-slate-600'>Mode</span>
								<span>{catalogDiagnostics.mode}</span>
								<span className='text-slate-600'>Duration</span>
								<span>{catalogDiagnostics.durationMs} ms</span>
								<span className='text-slate-600'>Applications</span>
								<span>{catalogDiagnostics.totalApps}</span>
								<span className='text-slate-600'>Changes</span>
								<span>
									+{catalogDiagnostics.added} / ~{catalogDiagnostics.updated} / -
									{catalogDiagnostics.removed}
								</span>
							</div>
							<p className='mt-3 text-xs leading-5 text-slate-600'>
								{Object.entries(catalogDiagnostics.sourceCounts)
									.map(([source, count]) => `${source}: ${count}`)
									.join(' · ')}
							</p>
							{catalogDiagnostics.visibilityCounts && (
								<p className='mt-1 text-xs leading-5 text-slate-600'>
									{Object.entries(catalogDiagnostics.visibilityCounts)
										.map(([visibility, count]) => `${visibility}: ${count}`)
										.join(' · ')}
								</p>
							)}
						</div>
					)}
					{(onRepairMissingIcons || onClearIconCache) && (
						<div className='mt-5 flex flex-wrap items-center gap-2 border-t border-slate-200/80 pt-4'>
							{onRepairMissingIcons && (
								<button
									type='button'
									disabled={iconAction !== null}
									onClick={() =>
										void runIconAction('repair', onRepairMissingIcons)
									}
									className='inline-flex items-center gap-2 rounded-lg border border-violet-300/60 px-3 py-2 text-sm text-violet-700 hover:bg-violet-100/70 disabled:opacity-50'
								>
									<Wrench size={15} aria-hidden='true' />
									{iconAction === 'repair' ? 'Repairing...' : 'Repair missing icons'}
								</button>
							)}
							{onClearIconCache && (
								<button
									type='button'
									disabled={iconAction !== null}
									onClick={() =>
										void runIconAction('clear', onClearIconCache)
									}
									className='inline-flex items-center gap-2 rounded-lg border border-slate-300/80 px-3 py-2 text-sm text-slate-700 hover:bg-violet-100/70 disabled:opacity-50'
								>
									<Image size={15} aria-hidden='true' />
									{iconAction === 'clear' ? 'Clearing...' : 'Clear icon cache'}
								</button>
							)}
							{iconMessage && (
								<span role='status' className='text-xs text-slate-600'>
									{iconMessage}
								</span>
							)}
						</div>
					)}
				</div>
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
