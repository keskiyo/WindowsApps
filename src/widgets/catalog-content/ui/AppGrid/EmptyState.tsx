import { SearchX } from 'lucide-react'

export function EmptyState({ hasQuery }: { hasQuery: boolean }) {
	return (
		<section className="grid min-h-[55vh] place-items-center text-center">
			<div className="max-w-sm">
				<SearchX
					className="mx-auto mb-5 text-slate-400"
					size={42}
					aria-hidden="true"
				/>
				<h2 className="text-lg font-semibold">
					{hasQuery ? 'No apps found' : 'No applications available'}
				</h2>
				<p className="mt-2 text-sm text-slate-600">
					{hasQuery
						? 'Try an app name, publisher, version, or install location.'
						: 'Refresh to scan Windows again.'}
				</p>
			</div>
		</section>
	)
}
