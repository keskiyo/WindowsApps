import { RefreshCw, RotateCcw, ScanSearch } from 'lucide-react'
import { useEffect, useRef } from 'react'
import type { MaintenanceConfirmation } from '../../../../features/edit-settings'
import { ScanDiagnostics } from './ScanDiagnostics'
import { DANGER_VARIANT } from '../../../../shared/ui/buttonVariants'
import {
	ACTION_BUTTON,
	ACTION_ROW,
	CANCEL_BUTTON,
	CONFIRM_BUTTON,
} from '../../data'
import type { CatalogMaintenanceProps } from '../../types'

export function CatalogMaintenance({
	forcing,
	resetting,
	confirming,
	canReset,
	catalogDiagnostics,
	visibilityCounts,
	setConfirming,
	onForceFullScan,
	onResetCatalogCache,
}: CatalogMaintenanceProps) {
	const forceTriggerRef = useRef<HTMLButtonElement>(null)
	const resetTriggerRef = useRef<HTMLButtonElement>(null)
	const previousConfirming = useRef<MaintenanceConfirmation>(null)
	useEffect(() => {
		const dismissed = previousConfirming.current
		previousConfirming.current = confirming
		if (!dismissed || dismissed === confirming) return
		if (confirming === null)
			(dismissed === 'force'
				? forceTriggerRef
				: resetTriggerRef
			).current?.focus()
	}, [confirming])

	return (
		<div className="settings-surface mt-5 rounded-2xl border border-white/85 bg-white/58 p-5">
			<div className="flex items-start gap-4">
				<span className="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner">
					<RefreshCw size={19} aria-hidden="true" />
				</span>
				<div className="min-w-0 flex-1">
					<h2 className="font-medium">Catalog maintenance</h2>
					<p className="mt-1 text-sm leading-6 text-slate-600">
						Discard the incremental scan index and inspect every
						configured location again. Categories, Favorites and
						Hidden apps are preserved.
					</p>
				</div>
			</div>
			<div className={`mt-4 ${ACTION_ROW}`}>
				<button
					ref={forceTriggerRef}
					type="button"
					disabled={forcing || resetting}
					onClick={() => setConfirming('force')}
					className={`${ACTION_BUTTON} utility-accent-button text-white focus-visible:outline-violet-400`}
				>
					<ScanSearch size={16} aria-hidden="true" />
					Force full scan
				</button>
				{canReset && (
					<button
						ref={resetTriggerRef}
						type="button"
						disabled={forcing || resetting}
						onClick={() => setConfirming('reset')}
						className={`${ACTION_BUTTON} ${DANGER_VARIANT}`}
					>
						<RotateCcw size={16} aria-hidden="true" />
						Reset catalog cache
					</button>
				)}
			</div>
			{confirming === 'force' && (
				<div
					role="dialog"
					aria-label="Confirm full scan"
					className="mt-4 flex flex-col gap-3 rounded-xl border border-violet-400/35 bg-violet-500/8 p-4 shadow-inner shadow-violet-950/10 sm:flex-row sm:items-center"
				>
					<p className="min-w-0 text-sm leading-6 text-slate-700 sm:flex-1">
						The next scan will take longer than an incremental
						refresh.
					</p>
					<div className={`${ACTION_ROW} sm:shrink-0`}>
						<button
							type="button"
							disabled={forcing}
							onClick={() => setConfirming(null)}
							className={CANCEL_BUTTON}
						>
							Cancel
						</button>
						<button
							type="button"
							disabled={forcing}
							onClick={() => void onForceFullScan()}
							className={`${CONFIRM_BUTTON} utility-accent-button text-white focus-visible:outline-violet-300`}
						>
							{forcing ? 'Scanning…' : 'Confirm full scan'}
						</button>
					</div>
				</div>
			)}
			{confirming === 'reset' && (
				<div
					role="dialog"
					aria-label="Confirm catalog cache reset"
					className="danger-panel mt-4 flex flex-col gap-3 rounded-xl border border-red-300/70 bg-red-50 p-4 sm:flex-row sm:items-center"
				>
					<p className="min-w-0 text-sm leading-6 text-red-800 sm:flex-1">
						This removes the local app cache and icon cache, then
						scans every configured location again. Favorites, Hidden
						apps and categories are preserved.
					</p>
					<div className={`${ACTION_ROW} sm:shrink-0`}>
						<button
							type="button"
							disabled={resetting}
							onClick={() => setConfirming(null)}
							className={CANCEL_BUTTON}
						>
							Cancel
						</button>
						<button
							type="button"
							disabled={resetting}
							onClick={() => void onResetCatalogCache()}
							className={`${CONFIRM_BUTTON} ${DANGER_VARIANT}`}
						>
							<RotateCcw size={16} aria-hidden="true" />
							{resetting ? 'Resetting…' : 'Confirm reset'}
						</button>
					</div>
				</div>
			)}
			{visibilityCounts && (
				<div className="mt-5 flex flex-wrap gap-x-6 gap-y-2 border-t border-slate-200/80 pt-4 text-sm">
					<span className="text-slate-600">Primary applications</span>
					<span className="font-medium text-slate-800">
						{visibilityCounts.primary}
					</span>
					<span className="text-slate-600">Auxiliary tools</span>
					<span className="font-medium text-slate-800">
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
