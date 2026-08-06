import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { MENU_ITEM_BASE } from './data'
import type { MenuItemProps } from './types'

export function MenuItem({
	icon: Icon,
	trailingIcon: TrailingIcon,
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
			? 'icon-follows-color text-rose-300 hover:bg-rose-400/15 hover:text-rose-200 focus-visible:outline-rose-300/70'
			: disabled
				? 'cursor-not-allowed text-slate-500'
				: 'text-slate-700 hover:bg-slate-500/15 focus-visible:outline-violet-500'

	return (
		<button
			type="button"
			role="menuitem"
			disabled={disabled}
			onClick={onClick}
			{...(withSpotlight ? spotlight : {})}
			className={`${MENU_ITEM_BASE} ${toneClass}`}
		>
			{withSpotlight && <SpotlightLayer size={70} />}
			{Icon && (
				<Icon size={15} className={iconClassName} aria-hidden="true" />
			)}
			{TrailingIcon ? (
				<span className="min-w-0 flex-1">{label}</span>
			) : (
				label
			)}
			{TrailingIcon && (
				<TrailingIcon
					size={15}
					className="shrink-0"
					aria-hidden="true"
				/>
			)}
		</button>
	)
}
