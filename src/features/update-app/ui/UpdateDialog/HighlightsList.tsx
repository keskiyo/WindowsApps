import { ExternalLink } from 'lucide-react'
import type { HighlightsListProps } from './types'

export function HighlightsList({
	highlights,
	releaseUrl,
	onOpenRelease,
}: HighlightsListProps) {
	return (
		<>
			<h3 className='text-sm font-semibold uppercase tracking-[0.12em] text-violet-200'>
				Highlights
			</h3>
			{highlights.length ? (
				<ul className='mt-4 space-y-3 text-sm leading-6 text-slate-200'>
					{highlights.map(item => (
						<li key={item} className='flex gap-3'>
							<span
								className='mt-2 size-1.5 shrink-0 rounded-full bg-violet-300'
								aria-hidden='true'
							/>
							<span>{item}</span>
						</li>
					))}
				</ul>
			) : (
				<p className='mt-4 rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-sm leading-6 text-slate-300'>
					Release notes are available in the latest GitHub release.
				</p>
			)}

			{releaseUrl && (
				<a
					href={releaseUrl}
					onClick={event => {
						event.preventDefault()
						onOpenRelease()
					}}
					className='mt-4 inline-flex items-center gap-2 text-sm font-medium text-violet-200 hover:text-violet-100 focus-visible:outline-2 focus-visible:outline-violet-300'
				>
					View full release notes
					<ExternalLink size={14} aria-hidden='true' />
				</a>
			)}
		</>
	)
}
