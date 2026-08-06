import { ArrowLeft } from 'lucide-react'
import type { CatalogViewHeaderProps } from '../types'

export function CatalogViewHeader({
	icon: Icon,
	title,
	titleId,
	count,
	noun = 'application',
	back,
}: CatalogViewHeaderProps) {
	return (
		<header className="mb-7 flex items-center gap-3">
			{back && (
				<button
					type="button"
					aria-label={back.label}
					onClick={back.onBack}
					className="grid size-9 shrink-0 place-items-center rounded-lg border border-(--border-neutral) bg-(--surface-panel) transition-[background-color,border-color] duration-200 hover:border-(--accent) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none"
				>
					<ArrowLeft size={18} aria-hidden="true" />
				</button>
			)}
			<Icon size={26} aria-hidden="true" className="shrink-0" />
			<div className="flex min-w-0 items-baseline gap-3">
				<h1
					id={titleId}
					className="truncate text-2xl font-semibold tracking-tight text-(--text-primary)"
				>
					{title}
				</h1>
				<span className="shrink-0 text-sm text-(--text-muted)">
					{count} {count === 1 ? noun : `${noun}s`}
				</span>
			</div>
		</header>
	)
}
