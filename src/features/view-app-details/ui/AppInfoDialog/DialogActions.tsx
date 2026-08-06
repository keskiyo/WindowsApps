import { Copy, FileText, FolderOpen, Loader2 } from 'lucide-react'
import type { DialogActionsProps } from './types'

export function DialogActions({
	canOpenFolder,
	isOpeningFolder,
	message,
	onCopyPath,
	onCopyReport,
	onOpenFolder,
}: DialogActionsProps) {
	const actionClass =
		'flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-1.5 rounded-xl border border-(--border-neutral) bg-(--surface-raised) px-3 py-2 text-sm font-medium text-(--text-primary) transition-colors hover:bg-[color-mix(in_oklab,var(--accent)_16%,var(--surface-raised))] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-not-allowed disabled:opacity-55 sm:flex-row'
	return (
		<div>
			<div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
				<button
					type="button"
					aria-label="Open folder"
					disabled={!canOpenFolder || isOpeningFolder}
					onClick={() => void onOpenFolder()}
					className={actionClass}
				>
					{isOpeningFolder ? (
						<Loader2
							size={18}
							className="animate-spin"
							aria-hidden="true"
						/>
					) : (
						<FolderOpen size={18} aria-hidden="true" />
					)}
					Open folder
				</button>
				<button
					type="button"
					aria-label="Copy path"
					onClick={() => void onCopyPath()}
					className={actionClass}
				>
					<Copy size={18} aria-hidden="true" />
					Copy path
				</button>
				<button
					type="button"
					aria-label="Copy report"
					onClick={() => void onCopyReport()}
					className={actionClass}
				>
					<FileText size={18} aria-hidden="true" />
					Copy report
				</button>
			</div>
			{message && (
				<p role="status" className="mt-2 text-sm text-(--text-muted)">
					{message}
				</p>
			)}
		</div>
	)
}
