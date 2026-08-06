import type { AppSidebarProps } from '../types'
import { AppNavigation } from './AppNavigation/AppNavigation'
import { NavigationIdentity } from './NavigationIdentity'

export function AppSidebar({ onGoHome, ...navigation }: AppSidebarProps) {
	return (
		<aside className="app-panel z-350 flex w-70 shrink-0 flex-col overflow-hidden rounded-2xl">
			<div className="border-b border-slate-300/65 px-4 py-4">
				<NavigationIdentity onGoHome={onGoHome} />
			</div>
			<AppNavigation {...navigation} />
		</aside>
	)
}
