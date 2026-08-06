import { AppWindow, X } from 'lucide-react'
import type { AppInfo } from '../../../entities/app'

interface Props {
	app: AppInfo
	remove?: { label: string; onRemove(): void }
}

export function ScenarioAppTile({ app, remove }: Props) {
	return (
		<li
			title={app.name}
			className="group/tile relative flex w-16 shrink-0 flex-col items-center gap-1 rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-1 pt-2 pb-1"
		>
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
