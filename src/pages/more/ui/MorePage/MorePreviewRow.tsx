import { AppWindow } from 'lucide-react'
import type { AppInfo } from '../../../../entities/app'
import { formatPreviewDate } from './data'
import type { MorePreviewItem } from './types'

/**
 * What the row can say under the name without inventing anything: the scanner's artifact verdict
 * where there is one, otherwise the publisher. An app with neither shows just its name.
 */
function subtitleOf(app: AppInfo): string | null {
	if (app.artifactKind === 'installer') return 'Installer'
	if (app.artifactKind === 'documentation') return 'Documentation'
	return app.publisher
}

export function MorePreviewRow({ entry }: { entry: MorePreviewItem }) {
	const { app } = entry
	const subtitle = subtitleOf(app)
	const firstSeenDate = formatPreviewDate(entry.firstSeenAt)

	return (
		<li className='flex min-w-0 items-center gap-3 border-t border-(--border-neutral) px-5 py-2.5 first:border-t-0'>
			<span className='grid size-8 shrink-0 place-items-center rounded-lg bg-(--surface-inset)'>
				{app.iconBase64 ? (
					<img
						src={app.iconBase64}
						alt=''
						className='size-6 object-contain'
					/>
				) : (
					<AppWindow size={16} aria-hidden='true' />
				)}
			</span>
			<span className='min-w-0 flex-1'>
				<span className='block truncate text-sm font-medium text-(--text-primary)'>
					{app.name}
				</span>
				{subtitle && (
					<span className='block truncate text-xs text-(--text-muted)'>
						{subtitle}
					</span>
				)}
			</span>
			{firstSeenDate && (
				<time
					dateTime={firstSeenDate.iso}
					aria-label={`First seen in catalog ${firstSeenDate.display}`}
					className='ml-auto shrink-0 whitespace-nowrap text-right text-xs tabular-nums text-(--text-muted)'
				>
					{firstSeenDate.display}
				</time>
			)}
		</li>
	)
}
