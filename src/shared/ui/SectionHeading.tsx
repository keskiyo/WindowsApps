import type { SectionHeadingProps } from './types'

export function SectionHeading({
	icon: Icon,
	title,
	titleId,
	count,
	noun,
	description,
}: SectionHeadingProps) {
	return (
		<header className="mb-3 flex items-start gap-3">
			<Icon size={20} aria-hidden="true" className="mt-0.5 shrink-0" />
			<div className="min-w-0">
				<div className="flex min-w-0 flex-wrap items-baseline gap-x-3">
					<h2
						id={titleId}
						className="truncate text-base font-semibold text-(--text-primary)"
					>
						{title}
					</h2>
					<span className="shrink-0 text-sm text-(--text-muted)">
						{count} {count === 1 ? noun : `${noun}s`}
					</span>
				</div>
				<p className="mt-0.5 text-sm text-(--text-muted)">
					{description}
				</p>
			</div>
		</header>
	)
}
