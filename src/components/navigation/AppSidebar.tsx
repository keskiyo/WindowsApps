import type { ComponentProps } from 'react'
import { AppNavigation } from './AppNavigation'

export function AppSidebar(props: ComponentProps<typeof AppNavigation>) {
	return (
		<aside className='fixed inset-y-0 left-0 z-350 flex w-70 flex-col border-r border-white/8 bg-slate-950'>
			<div className='border-b border-white/8 px-5 py-5'>
				<p className='text-sm font-semibold text-slate-100'>Library</p>
				<p className='mt-1 text-xs text-slate-500'>
					Navigate your applications
				</p>
			</div>
			<AppNavigation {...props} />
		</aside>
	)
}
