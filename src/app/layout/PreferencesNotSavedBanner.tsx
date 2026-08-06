import { TriangleAlert } from 'lucide-react'

export function PreferencesNotSavedBanner() {
	return (
		<div
			role="status"
			className="flex items-center gap-3 border-b border-amber-400/30 bg-amber-500/12 px-4 py-2 text-sm text-slate-200"
		>
			<TriangleAlert
				size={16}
				className="shrink-0 text-amber-300"
				aria-hidden="true"
			/>
			<span className="min-w-0 flex-1">
				<span className="font-medium">
					Your changes are not being saved
				</span>
				<span className="text-slate-400">
					{' '}
					— browser storage is full or unavailable, so favorites,
					hidden apps and custom categories will be lost when Windows
					Apps restarts.
				</span>
			</span>
		</div>
	)
}
