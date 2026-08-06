import { Star } from 'lucide-react'
import type { FavoriteButtonProps } from './types'

export function FavoriteButton({
	appName,
	isFavorite,
	onToggle,
}: FavoriteButtonProps) {
	return (
		<button
			type='button'
			aria-label={`${isFavorite ? 'Remove' : 'Add'} ${appName} ${isFavorite ? 'from' : 'to'} favorites`}
			aria-pressed={isFavorite}
			onClick={event => {
				event.stopPropagation()
				onToggle()
			}}
			className={`icon-follows-color absolute right-2 top-2 z-2 grid size-8 place-items-center rounded-lg transition focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-yellow-300 ${isFavorite ? 'text-yellow-300 opacity-100' : 'bg-transparent text-slate-400 opacity-0 hover:text-yellow-300 group-hover:opacity-100 group-focus-within:opacity-100'}`}
		>
			<Star
				size={16}
				fill={isFavorite ? 'currentColor' : 'none'}
				aria-hidden='true'
			/>
		</button>
	)
}
