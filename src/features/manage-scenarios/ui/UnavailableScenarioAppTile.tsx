import { AppWindow, X } from 'lucide-react'
import type { UnavailableScenarioAppTileProps } from '../types'

export function UnavailableScenarioAppTile({
	entry,
	remove,
}: UnavailableScenarioAppTileProps) {
	return (
		<li
			title={`${entry.name} — Unavailable`}
			className="group/tile relative flex w-16 shrink-0 flex-col items-center gap-1 rounded-lg border border-dashed border-(--border-neutral) bg-(--surface-inset) px-1 pt-2 pb-1 opacity-75"
		>
			<span className="grid size-8 place-items-center">
				{entry.iconBase64 ? (
					<img
						src={entry.iconBase64}
						alt=""
						className="size-7 object-contain grayscale"
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
				{entry.name}
			</span>
			<span className="text-[0.5625rem] leading-3 text-(--text-muted)">
				Unavailable
			</span>
			{remove && (
				<button
					type="button"
					aria-label={remove.label}
					disabled={remove.disabled}
					onClick={remove.onRemove}
					className="absolute -top-1.5 -right-1.5 grid size-5 place-items-center rounded-full border border-(--border-neutral) bg-(--surface-raised) opacity-0 transition-opacity group-focus-within/tile:opacity-100 group-hover/tile:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-(--accent-strong) disabled:cursor-not-allowed disabled:opacity-60 motion-reduce:transition-none"
				>
					<X size={12} aria-hidden="true" />
				</button>
			)}
		</li>
	)
}
