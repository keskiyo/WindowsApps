import type { ComponentProps } from 'react'
import { AppNavigation } from './AppNavigation'

export function AppSidebar(props: ComponentProps<typeof AppNavigation>) {
	return (
		<aside className='app-panel z-350 flex w-70 shrink-0 flex-col overflow-hidden rounded-2xl'>
			<div className='border-b border-slate-300/65 px-5 py-5'>
				<p className='text-sm font-semibold text-slate-800'>Library</p>
				<p className='mt-1 text-xs text-slate-500'>
					Navigate your applications
				</p>
			</div>
			<AppNavigation {...props} />
		</aside>
	)
}
