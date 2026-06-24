import type { ComponentProps } from 'react'
import { AppNavigation } from './AppNavigation'

export function AppSidebar(props: ComponentProps<typeof AppNavigation>) {
	return (
		<aside className='fixed inset-y-0 left-0 z-350 flex w-70 flex-col border-r border-slate-300/65 bg-slate-50/90 shadow-[8px_0_28px_rgba(73,82,105,.06)] backdrop-blur-xl'>
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
