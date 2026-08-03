import { RefreshCw, RotateCcw, ScanSearch } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { ScanDiagnostics } from './ScanDiagnostics'
import type { CatalogMaintenanceProps } from '../../types'

/**
 * One geometry for every control in this card, so a variant differs by colour alone. The two
 * actions previously drifted — only one carried an icon, and the reset dialog's Cancel had neither
 * a disabled style nor a focus ring, so it looked enabled while a reset was running.
 *
 * Colour still separates them on purpose: violet is the ordinary action, red the destructive one.
 * That distinction is a safety signal, not an inconsistency.
 */
const ACTION_BUTTON =
	'inline-flex min-h-11 items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50'

const CONFIRM_BUTTON =
	'inline-flex min-h-10 items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50'

/** Neutral dismiss, shared by both confirmation dialogs. */
const CANCEL_BUTTON = `${CONFIRM_BUTTON} border border-slate-300/80 bg-white/60 text-slate-700 hover:bg-violet-100/70 focus-visible:outline-violet-400`

/**
 * The destructive look, shared by the trigger and the confirmation that commits it, so the two
 * cannot drift apart again — the confirmation used to be a saturated `bg-red-500` fill next to an
 * outlined trigger. `.danger-button` carries the dark-theme treatment (muted red field, red border,
 * light red text); the surrounding red panel already signals severity without a loud fill.
 */
const DANGER_VARIANT =
	'danger-button border border-red-300/70 text-red-700 hover:bg-red-100 focus-visible:outline-red-400'

/**
 * Stacked and full width at the minimum window size; once there is room they sit side by side and
 * align to the right edge, which is where the card's actions belong now that they are below the
 * description rather than beside it.
 */
const ACTION_ROW = 'grid gap-2 sm:flex sm:flex-wrap sm:justify-end'

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
			<div className='flex items-start gap-4'>
				<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
					<RefreshCw size={19} aria-hidden='true' />
				</span>
				<div className='min-w-0 flex-1'>
					<h2 className='font-medium'>Catalog maintenance</h2>
					<p className='mt-1 text-sm leading-6 text-slate-600'>
						Discard the incremental scan index and inspect every
						configured location again. Categories, Favorites and
						Hidden apps are preserved.
					</p>
				</div>
			</div>
			{/* Below the description rather than beside it, so the text keeps the card's full
			    width instead of being squeezed into a narrow column by the buttons. */}
			<div className={`mt-4 ${ACTION_ROW}`}>
				<button
					ref={forceTriggerRef}
					type='button'
					disabled={forcing || resetting}
					onClick={() => setConfirmForce(true)}
					className={`${ACTION_BUTTON} utility-accent-button text-white shadow-[var(--shadow-accent-soft)] focus-visible:outline-violet-400`}
				>
					<ScanSearch size={16} aria-hidden='true' />
					Force full scan
				</button>
				{canReset && (
					<button
						ref={resetTriggerRef}
						type='button'
						disabled={forcing || resetting}
						onClick={() => setConfirmReset(true)}
						className={`${ACTION_BUTTON} ${DANGER_VARIANT}`}
					>
						<RotateCcw size={16} aria-hidden='true' />
						Reset catalog cache
					</button>
				)}
			</div>
			{confirmForce && (
				<div
					role='dialog'
					aria-label='Confirm full scan'
					className='mt-4 flex flex-col gap-3 rounded-xl border border-violet-400/35 bg-violet-500/8 p-4 shadow-inner shadow-violet-950/10 sm:flex-row sm:items-center'
				>
					<p className='min-w-0 text-sm leading-6 text-slate-700 sm:flex-1'>
						The next scan will take longer than an incremental
						refresh.
					</p>
					<div className={`${ACTION_ROW} sm:shrink-0`}>
						<button
							type='button'
							disabled={forcing}
							onClick={() => setConfirmForce(false)}
							className={CANCEL_BUTTON}
						>
							Cancel
						</button>
						<button
							type='button'
							disabled={forcing}
							onClick={() => void onForceFullScan()}
							className={`${CONFIRM_BUTTON} utility-accent-button text-white shadow-[var(--shadow-accent-vivid)] focus-visible:outline-violet-300`}
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
					className='danger-panel mt-4 flex flex-col gap-3 rounded-xl border border-red-300/70 bg-red-50 p-4 sm:flex-row sm:items-center'
				>
					<p className='min-w-0 text-sm leading-6 text-red-800 sm:flex-1'>
						This removes the local app cache and icon cache, then
						scans every configured location again. Favorites, Hidden
						apps and categories are preserved.
					</p>
					<div className={`${ACTION_ROW} sm:shrink-0`}>
						<button
							type='button'
							disabled={resetting}
							onClick={() => setConfirmReset(false)}
							className={CANCEL_BUTTON}
						>
							Cancel
						</button>
						<button
							type='button'
							disabled={resetting}
							onClick={() => void onResetCatalogCache()}
							className={`${CONFIRM_BUTTON} ${DANGER_VARIANT}`}
						>
							<RotateCcw size={16} aria-hidden='true' />
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
