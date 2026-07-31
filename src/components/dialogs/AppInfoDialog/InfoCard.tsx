import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'

interface InfoCardProps {
	icon: LucideIcon
	title: string
	children: ReactNode
}

export function InfoCard({ icon: Icon, title, children }: InfoCardProps) {
	return (
		<section className='rounded-2xl border border-[var(--border-neutral)] bg-[var(--surface-inset)] p-4 shadow-[var(--shadow-summary)]'>
			<div className='mb-3 flex items-center gap-3'>
				<span className='grid size-10 place-items-center rounded-xl border border-[var(--border-neutral)] bg-[var(--surface-raised)] text-[var(--accent-strong)]'>
					<Icon size={20} aria-hidden='true' />
				</span>
				<h3 className='text-base font-semibold text-[var(--text-primary)]'>
					{title}
				</h3>
			</div>
			{children}
		</section>
	)
}
