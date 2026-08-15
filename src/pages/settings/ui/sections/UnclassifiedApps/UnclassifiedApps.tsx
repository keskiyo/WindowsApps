import { ChevronDown, ClipboardCopy, HelpCircle } from 'lucide-react'
import { useState } from 'react'
import { CollapsiblePanel } from '../../../../../shared/ui/CollapsiblePanel'
import { copyToClipboard } from '../../../../../shared/lib/clipboard'
import { ACTION_BUTTON } from '../../../data'
import { buildDiagnosticsReport } from './data'
import { UnclassifiedRow } from './UnclassifiedRow'
import type { UnclassifiedAppsProps } from './types'

export function UnclassifiedApps({
	apps,
	categories,
	categoryOrder,
	onMoveApp,
}: UnclassifiedAppsProps) {
	const [expanded, setExpanded] = useState(false)
	const [message, setMessage] = useState<string | null>(null)
	const contentId = 'unclassified-apps'
	if (apps.length === 0) return null

	async function copyReport() {
		try {
			await copyToClipboard(buildDiagnosticsReport(apps))
			setMessage('Diagnostics copied.')
		} catch {
			setMessage('Could not copy the diagnostics.')
		}
	}

	return (
		<div className="settings-surface rounded-2xl border border-white/85 bg-white/58 p-5">
			<button
				type="button"
				aria-expanded={expanded}
				aria-controls={contentId}
				onClick={() => setExpanded(value => !value)}
				className="-mx-2 flex w-full items-center gap-4 rounded-xl px-2 py-1 text-left transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none"
			>
				<span className="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner">
					<HelpCircle size={19} aria-hidden="true" />
				</span>
				<span className="min-w-0 flex-1">
					<span className="block font-medium">
						Unrecognised applications ({apps.length})
					</span>
					<span className="mt-1 block text-sm leading-6 text-slate-600">
						No rule matched these records, so they stay in Other.
						Every signal the classifier read is listed beside each
						one.
					</span>
				</span>
				<ChevronDown
					size={18}
					aria-hidden="true"
					className={`shrink-0 transition-transform duration-(--motion-fast) motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
				/>
			</button>
			<CollapsiblePanel open={expanded} id={contentId}>
				<div className="mt-4 flex flex-col gap-3">
					<div className="flex items-center justify-end gap-3">
						<p
							role="status"
							aria-live="polite"
							className="min-w-0 truncate text-sm text-slate-600"
						>
							{message}
						</p>
						<button
							type="button"
							onClick={() => void copyReport()}
							className={`${ACTION_BUTTON} utility-accent-button shrink-0 text-white focus-visible:outline-violet-400`}
						>
							<ClipboardCopy size={16} aria-hidden="true" />
							Copy diagnostics
						</button>
					</div>
					<ul className="flex flex-col gap-2">
						{apps.map(app => (
							<UnclassifiedRow
								key={app.id}
								app={app}
								categories={categories}
								categoryOrder={categoryOrder}
								onMoveApp={onMoveApp}
							/>
						))}
					</ul>
				</div>
			</CollapsiblePanel>
		</div>
	)
}
