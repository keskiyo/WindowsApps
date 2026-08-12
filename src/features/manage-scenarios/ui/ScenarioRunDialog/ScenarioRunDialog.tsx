import { ListChecks, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { sortScenariosByNewest } from '../../../../entities/scenario'
import { useBodyScrollLock } from '../../../../shared/hooks/useBodyScrollLock'
import { useFocusTrap } from '../../../../shared/hooks/useFocusTrap'
import { DIALOG_LABEL } from './data'
import { ScenarioRunRow } from './ScenarioRunRow'
import type { ScenarioRunDialogProps } from './types'

export function ScenarioRunDialog({
	scenarios,
	apps,
	runningId,
	isScenarioRunning,
	onRun,
	onClose,
}: ScenarioRunDialogProps) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLDivElement>(null)
	const closeRef = useRef<HTMLButtonElement>(null)
	const [expanded, setExpanded] = useState<string[]>([])
	useFocusTrap(dialogRef)

	useEffect(() => {
		const trigger = document.activeElement
		closeRef.current?.focus()
		return () => {
			if (trigger instanceof HTMLElement && trigger.isConnected)
				trigger.focus()
		}
	}, [])

	function toggle(id: string) {
		setExpanded(open =>
			open.includes(id)
				? open.filter(entry => entry !== id)
				: [...open, id],
		)
	}

	return (
		<div
			className="motion-overlay fixed inset-0 z-500 grid place-items-start justify-center bg-slate-700/40 px-4 pt-[7vh] backdrop-blur-[2px]"
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<div
				ref={dialogRef}
				role="dialog"
				aria-modal="true"
				aria-label={DIALOG_LABEL}
				onKeyDown={event => {
					if (event.key !== 'Escape') return
					event.preventDefault()
					onClose()
				}}
				className="motion-panel flex max-h-[min(48rem,calc(100vh-4rem))] w-full min-w-xl flex-col overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-panel) shadow-(--shadow-palette)"
			>
				<div className="flex items-center gap-3 border-b border-(--border-neutral) px-4 py-3">
					<ListChecks size={18} aria-hidden="true" />
					<h2 className="min-w-0 flex-1 truncate text-sm font-semibold text-(--text-primary)">
						{DIALOG_LABEL}
					</h2>
					<button
						ref={closeRef}
						type="button"
						aria-label="Close all scenarios"
						onClick={onClose}
						className="grid size-8 shrink-0 place-items-center rounded-lg hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
					>
						<X size={16} aria-hidden="true" />
					</button>
				</div>
				{scenarios.length === 0 ? (
					<p className="px-4 py-8 text-center text-sm text-(--text-muted)">
						No scenarios yet.
					</p>
				) : (
					<ul
						aria-label="Scenarios"
						className="min-h-0 flex-1 overflow-y-auto overscroll-contain"
					>
						{sortScenariosByNewest(scenarios).map(scenario => (
							<ScenarioRunRow
								key={scenario.id}
								scenario={scenario}
								apps={apps}
								expanded={expanded.includes(scenario.id)}
								running={runningId === scenario.id}
								isScenarioRunning={isScenarioRunning}
								onToggle={toggle}
								onRun={onRun}
							/>
						))}
					</ul>
				)}
			</div>
		</div>
	)
}
