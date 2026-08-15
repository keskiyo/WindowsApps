import { AppWindow } from 'lucide-react'
import { formatPreviewDate } from './data'
import { previewSubtitleOf } from './previewSubtitle'
import type { MorePreviewRowProps } from './types'

export function MorePreviewRow({ entry }: MorePreviewRowProps) {
	const { app } = entry
	const subtitle = previewSubtitleOf(app)
	const firstSeenDate = formatPreviewDate(entry.firstSeenAt)

	return (
		<li className="flex min-w-0 flex-auto items-center gap-3 border-t border-(--border-neutral) px-5 py-2.5 first:border-t-0">
			<span className="grid size-8 shrink-0 place-items-center rounded-lg bg-(--surface-inset)">
				{app.iconBase64 ? (
					<img
						src={app.iconBase64}
						alt=""
						className="size-6 object-contain"
					/>
				) : (
					<AppWindow size={16} aria-hidden="true" />
				)}
			</span>
			<span className="min-w-0 flex-1">
				<span className="block truncate text-sm font-medium text-(--text-primary)">
					{app.name}
				</span>
				{subtitle && (
					<span className="block truncate text-xs text-(--text-muted)">
						{subtitle}
					</span>
				)}
			</span>
			{firstSeenDate && (
				<time
					dateTime={firstSeenDate.iso}
					aria-label={`First seen in catalog ${firstSeenDate.display}`}
					className="ml-auto shrink-0 text-right text-xs whitespace-nowrap text-(--text-muted) tabular-nums"
				>
					{firstSeenDate.display}
				</time>
			)}
		</li>
	)
}
