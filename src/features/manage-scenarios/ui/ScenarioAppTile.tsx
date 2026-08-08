import { AppWindow, TriangleAlert, X } from 'lucide-react'
import type { ScenarioAppTileProps } from '../types'

export function ScenarioAppTile({ app, remove, mark }: ScenarioAppTileProps) {
	const danger = mark?.badge.tone === 'danger'
	return (
		<li
			title={mark ? `${app.name} — ${mark.reason}` : app.name}
			className={`group/tile relative flex w-16 shrink-0 flex-col items-center gap-1 rounded-lg border bg-(--surface-panel) px-1 pt-2 pb-1 ${danger ? 'border-(--category-orange)' : 'border-(--border-neutral)'}`}
		>
			{mark && (
				<span
					role="img"
					aria-label={`${app.name}: ${mark.badge.label} — ${mark.reason}`}
					className="absolute -top-1.5 -left-1.5 grid size-5 place-items-center rounded-full border border-(--border-neutral) bg-(--surface-raised)"
				>
					<TriangleAlert size={11} aria-hidden="true" />
				</span>
			)}
			<span className="grid size-8 place-items-center">
				{app.iconBase64 ? (
					<img
						src={app.iconBase64}
						alt=""
						className="size-7 object-contain"
						draggable={false}
					/>
				) : (
					<AppWindow
						size={20}
						aria-hidden="true"
						className="text-(--text-muted)"
					/>
				)}
			</span>
			<span className="w-full truncate text-center text-[0.6875rem] leading-4 text-(--text-primary)">
				{app.name}
			</span>
			{remove && (
				<button
					type="button"
					aria-label={remove.label}
					onClick={remove.onRemove}
					className="absolute -top-1.5 -right-1.5 grid size-5 place-items-center rounded-full border border-(--border-neutral) bg-(--surface-raised) opacity-0 transition-opacity group-focus-within/tile:opacity-100 group-hover/tile:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-(--accent-strong) motion-reduce:transition-none"
				>
					<X size={12} aria-hidden="true" />
				</button>
			)}
		</li>
	)
}
