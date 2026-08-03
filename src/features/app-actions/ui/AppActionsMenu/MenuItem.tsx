import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import type { MenuItemProps } from './types'

const BASE =
	'relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm focus-visible:outline-2'

export function MenuItem({
	icon: Icon,
	label,
	onClick,
	tone = 'default',
	disabled = false,
	iconClassName,
	withSpotlight = true,
}: MenuItemProps) {
	const spotlight = useSpotlight()
	const toneClass =
		tone === 'danger'
			? 'text-rose-300 hover:bg-rose-400/15 hover:text-rose-200 focus-visible:outline-rose-300/70'
			: disabled
				? 'cursor-not-allowed text-slate-500'
				: 'text-slate-700 hover:bg-slate-500/15 focus-visible:outline-violet-500'

	return (
		<button
			type='button'
			role='menuitem'
			disabled={disabled}
			onClick={onClick}
			{...(withSpotlight ? spotlight : {})}
			className={`${BASE} ${toneClass}`}
		>
			{withSpotlight && <SpotlightLayer size={70} />}
			<Icon size={15} className={iconClassName} aria-hidden='true' />
			{label}
		</button>
	)
}
