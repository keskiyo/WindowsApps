import type { AppInfo } from '../../model/app.types'
import { CardIcon } from './CardIcon'
import { CardLabel } from './CardLabel'

interface Props {
	app: AppInfo
}

export function AppDragOverlay({ app }: Props) {
	return (
		<article
			aria-hidden='true'
			data-testid='app-drag-overlay'
			className='app-card app-card-glass group pointer-events-none relative flex min-h-34 flex-col items-center justify-center gap-3 rounded-[1.15rem] border border-white/85 px-4 py-4 text-center shadow-[var(--shadow-menu)]'
		>
			<CardIcon iconBase64={app.iconBase64} launching={false} />
			<CardLabel
				name={app.name}
				version={app.version}
				launching={false}
			/>
		</article>
	)
}
