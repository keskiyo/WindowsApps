import type { DetailRowsProps } from './types'

export function DetailRows({ rows }: DetailRowsProps) {
	return (
		<dl className="grid grid-cols-[minmax(6.5rem,auto)_minmax(0,1fr)] gap-x-3 gap-y-2 text-sm leading-5">
			{rows.map(([label, value]) => (
				<div key={label} className="contents">
					<dt className="text-(--text-subtle)">{label}</dt>
					<dd className="min-w-0 break-words text-(--text-primary)">
						{value}
					</dd>
				</div>
			))}
		</dl>
	)
}
