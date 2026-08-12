import { ChevronDown, SlidersHorizontal } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { CollapsiblePanel } from '../../../../shared/ui/CollapsiblePanel'

export function AdvancedSettings({ children }: { children: ReactNode }) {
	const [expanded, setExpanded] = useState(false)
	const contentId = 'advanced-settings'
	return (
		<section className="mt-5">
			<button
				type="button"
				aria-expanded={expanded}
				aria-controls={contentId}
				onClick={() => setExpanded(value => !value)}
				className="settings-surface flex min-h-16 w-full items-center gap-4 rounded-2xl border border-white/85 bg-white/58 p-5 text-left transition-colors hover:bg-white/70 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
			>
				<span className="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner">
					<SlidersHorizontal size={19} aria-hidden="true" />
				</span>
				<span className="min-w-0 flex-1">
					<span className="block font-medium">Advanced</span>
					<span className="mt-1 block text-sm text-slate-600">
						Catalog scanning, backups, and local history.
					</span>
				</span>
				<ChevronDown
					size={18}
					aria-hidden="true"
					className={`shrink-0 transition-transform duration-(--motion-fast) motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
				/>
			</button>
			<CollapsiblePanel open={expanded} id={contentId} className="pt-5">
				<div className="space-y-5">{children}</div>
			</CollapsiblePanel>
		</section>
	)
}
