import { Star } from 'lucide-react'
import type { FavoriteStarProps } from './types'

export function FavoriteStar({
	label,
	pressed,
	className = '',
	onToggle,
}: FavoriteStarProps) {
	return (
		<button
			type="button"
			aria-label={label}
			aria-pressed={pressed}
			onClick={event => {
				event.stopPropagation()
				onToggle()
			}}
			className={`icon-follows-color grid size-8 place-items-center rounded-lg transition focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-yellow-300 ${pressed ? 'text-yellow-300 opacity-100' : 'bg-transparent text-slate-400 hover:text-yellow-300'} ${className}`}
		>
			<Star
				size={16}
				fill={pressed ? 'currentColor' : 'none'}
				aria-hidden="true"
			/>
		</button>
	)
}
