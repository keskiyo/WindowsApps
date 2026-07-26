import { RefreshCw, RotateCcw } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { ScanDiagnostics } from './ScanDiagnostics'
import type { CatalogMaintenanceProps } from './types'

export function CatalogMaintenance({
	forcing,
	resetting,
	confirmForce,
	confirmReset,
	canReset,
	catalogDiagnostics,
	visibilityCounts,
	setConfirmForce,
	setConfirmReset,
	onForceFullScan,
	onResetCatalogCache,
}: CatalogMaintenanceProps) {
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

	return (
		<div className='settings-surface mt-5 rounded-2xl border border-white/85 bg-white/58 p-5'>
			<div className='flex flex-wrap items-center gap-4'>
				<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
					<RefreshCw size={19} aria-hidden='true' />
				</span>
				<div className='min-w-60 flex-1'>
					<h2 className='font-medium'>Catalog maintenance</h2>
					<p className='mt-1 text-sm leading-6 text-slate-600'>
						Discard the incremental scan index and inspect every
						configured location again. Categories, Favorites and
						Hidden apps are preserved.
					</p>
				</div>
				<div className='ml-auto flex shrink-0 flex-wrap gap-2'>
					<button
						ref={forceTriggerRef}
						type='button'
						disabled={forcing || resetting}
						onClick={() => setConfirmForce(true)}
						className='utility-accent-button rounded-xl px-4 py-2.5 text-sm font-medium text-white shadow-[var(--shadow-accent-soft)] disabled:opacity-50'
					>
						Force full scan
					</button>
					{canReset && (
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
						The next scan will take longer than an incremental
						refresh.
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
							onClick={() => void onForceFullScan()}
							className='utility-accent-button rounded-lg px-3 py-2 text-sm font-medium text-white shadow-[var(--shadow-accent-vivid)] transition-colors focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-50'
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
					className='danger-panel mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-red-300/70 bg-red-50 p-4'
				>
					<p className='text-sm text-red-800'>
						This removes the local app cache and icon cache, then
						scans every configured location again. Favorites, Hidden
						apps and categories are preserved.
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
							onClick={() => void onResetCatalogCache()}
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
					<span className='font-medium text-slate-800'>
						{visibilityCounts.primary}
					</span>
					<span className='text-slate-600'>Auxiliary tools</span>
					<span className='font-medium text-slate-800'>
						{visibilityCounts.auxiliary}
					</span>
				</div>
			)}
			{catalogDiagnostics && (
				<ScanDiagnostics diagnostics={catalogDiagnostics} />
			)}
		</div>
	)
}
