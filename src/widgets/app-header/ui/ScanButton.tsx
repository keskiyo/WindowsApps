import { RefreshCw, X } from 'lucide-react'
import { useSpotlight } from '../../../hooks/useSpotlight'
import { SpotlightLayer } from '../SpotlightLayer'
import type { ScanButtonProps } from './types'

export function ScanButton({
	isRefreshing,
	onRefresh,
	onCancelScan,
}: ScanButtonProps) {
	const scanSpotlight = useSpotlight()

	return (
		<button
			type='button'
			onClick={() => void (isRefreshing ? onCancelScan() : onRefresh())}
			aria-label={isRefreshing ? 'Cancel scan' : 'Scan for apps'}
			onPointerMove={scanSpotlight.onPointerMove}
			onPointerEnter={scanSpotlight.onPointerEnter}
			onPointerLeave={scanSpotlight.onPointerLeave}
			className={`relative grid size-11 shrink-0 place-items-center rounded-xl text-white shadow-(--shadow-accent-soft) focus-visible:outline-2 ${isRefreshing ? 'bg-red-500 hover:bg-red-400 focus-visible:outline-red-300' : 'utility-accent-button focus-visible:outline-violet-500'}`}
		>
			<SpotlightLayer size={70} />
			{isRefreshing ? <X size={18} /> : <RefreshCw size={18} />}
		</button>
	)
}
