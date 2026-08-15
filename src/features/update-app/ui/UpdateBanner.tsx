import { ArrowUpCircle, Sparkles, X } from 'lucide-react'
import type { UpdateBannerProps } from './UpdateDialog/types'

export function UpdateBanner({
	version,
	onOpen,
	onDismiss,
}: UpdateBannerProps) {
	return (
		<div
			role="status"
			className="flex items-center gap-3 border-b border-violet-400/30 bg-violet-500/12 px-4 py-2 text-sm text-slate-200"
		>
			<Sparkles
				size={16}
				className="shrink-0 text-violet-300"
				aria-hidden="true"
			/>
			<span className="min-w-0 flex-1 truncate">
				<span className="font-medium">
					Version {version} is available
				</span>
			</span>
			<button
				type="button"
				onClick={onOpen}
				className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-violet-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-violet-400 focus-visible:outline-2 focus-visible:outline-violet-300"
			>
				<ArrowUpCircle size={14} aria-hidden="true" />
				View update
			</button>
			<button
				type="button"
				aria-label="Dismiss update notice"
				onClick={onDismiss}
				className="grid size-7 shrink-0 place-items-center rounded-lg text-slate-400 hover:bg-(--surface-raised) hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-violet-300"
			>
				<X size={15} />
			</button>
		</div>
	)
}
