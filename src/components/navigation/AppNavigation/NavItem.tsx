import { useSpotlight } from '../../../hooks/useSpotlight'
import { SpotlightLayer } from '../../shared/SpotlightLayer'
import type { NavItemProps } from './types'

function itemClass(active: boolean) {
	return `relative flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-colors focus-visible:outline-2 focus-visible:outline-violet-500 ${active ? 'bg-violet-100/90 font-medium text-violet-700 shadow-[var(--shadow-active-navigation)]' : 'text-slate-600 hover:bg-violet-100/65 hover:text-violet-700'}`
}

export function NavItem({
	icon: Icon,
	label,
	active,
	count,
	className,
	onClick,
}: NavItemProps) {
	const spotlight = useSpotlight()

	return (
		<button
			type='button'
			onClick={onClick}
			aria-current={active ? 'page' : undefined}
			{...spotlight}
			className={`${itemClass(active)}${className ? ` ${className}` : ''}`}
		>
			<SpotlightLayer size={90} />
			<Icon size={17} /> <span>{label}</span>
			{count !== undefined && (
				<span className='ml-auto rounded-full bg-slate-200/85 px-2 py-0.5 text-xs text-slate-600'>
					{count}
				</span>
			)}
		</button>
	)
}
