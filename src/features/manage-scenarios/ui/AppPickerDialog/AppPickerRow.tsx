import { AppWindow, TriangleAlert } from 'lucide-react'
import { ToggleTrack } from '../../../../shared/ui/ToggleTrack'
import type { AppPickerRowProps } from './types'

export function AppPickerRow({
	app,
	caption,
	checked,
	disabled,
	mark,
	onToggle,
}: AppPickerRowProps) {
	return (
		<li className="picker-row">
			<button
				type="button"
				role="switch"
				aria-checked={checked}
				disabled={disabled}
				title={mark?.reason}
				onClick={onToggle}
				className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) ${disabled ? 'cursor-not-allowed text-(--text-muted) opacity-70' : 'text-(--text-primary) hover:bg-(--surface-raised)'}`}
			>
				<span className="grid size-7 shrink-0 place-items-center rounded-md bg-(--surface-inset)">
					{app.iconBase64 ? (
						<img
							src={app.iconBase64}
							alt=""
							className="size-5 object-contain"
						/>
					) : (
						<AppWindow
							size={15}
							aria-hidden="true"
							className="text-(--text-muted)"
						/>
					)}
				</span>
				<span className="min-w-0 flex-1">
					<span className="block truncate">{app.name}</span>
					<span className="block truncate text-xs text-(--text-subtle)">
						{caption}
					</span>
				</span>
				{mark && (
					<span
						className={`inline-flex shrink-0 items-center gap-1 rounded-md border px-1.5 py-0.5 text-[0.6875rem] ${mark.badge.tone === 'danger' ? 'border-(--category-orange) text-(--text-primary)' : 'border-(--border-neutral) text-(--text-muted)'}`}
					>
						{mark.badge.tone === 'danger' && (
							<TriangleAlert size={11} aria-hidden="true" />
						)}
						{mark.badge.label}
					</span>
				)}
				<ToggleTrack checked={checked} />
			</button>
		</li>
	)
}
