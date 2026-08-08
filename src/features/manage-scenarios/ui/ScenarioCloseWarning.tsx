import { TriangleAlert } from 'lucide-react'
import type { ScenarioCloseWarningProps } from '../types'

export function ScenarioCloseWarning({
	appName,
	message,
	onConfirm,
	onCancel,
}: ScenarioCloseWarningProps) {
	return (
		<div
			role="alertdialog"
			aria-label={`Add ${appName} to the close list`}
			className="flex flex-col gap-2 rounded-xl border border-(--category-orange) bg-(--surface-inset) p-3"
		>
			<p className="flex items-start gap-2 text-xs leading-5 text-(--text-primary)">
				<TriangleAlert
					size={15}
					aria-hidden="true"
					className="mt-0.5 shrink-0"
				/>
				<span>
					<strong className="font-semibold">{appName}</strong> —{' '}
					{message}
				</span>
			</p>
			<div className="flex flex-wrap justify-end gap-2">
				<button
					type="button"
					onClick={onCancel}
					className="inline-flex h-8 items-center rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
				>
					Cancel
				</button>
				<button
					type="button"
					onClick={onConfirm}
					className="inline-flex h-8 items-center rounded-lg border border-(--category-orange) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
				>
					Add anyway
				</button>
			</div>
		</div>
	)
}
