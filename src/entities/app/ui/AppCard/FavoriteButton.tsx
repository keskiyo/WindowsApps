import { FavoriteStar } from '../../../../shared/ui/FavoriteStar'
import type { FavoriteButtonProps } from './types'

export function FavoriteButton({
	appName,
	isFavorite,
	onToggle,
}: FavoriteButtonProps) {
	return (
		<FavoriteStar
			label={`${isFavorite ? 'Remove' : 'Add'} ${appName} ${isFavorite ? 'from' : 'to'} favorites`}
			pressed={isFavorite}
			onToggle={onToggle}
			className={`absolute top-2 right-2 z-2 ${isFavorite ? '' : 'opacity-0 group-focus-within:opacity-100 group-hover:opacity-100'}`}
		/>
	)
}
