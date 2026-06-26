import { getCurrentWindow } from '@tauri-apps/api/window'
import { Copy, Minus, Square, X } from 'lucide-react'
import { useEffect, useState } from 'react'

/**
 * Custom window title bar (native decorations are disabled) so the very top matches the
 * app's surface instead of the OS chrome. The bar is the drag region; the buttons drive
 * the window via Tauri's window API. The maximize button reflects the current window state
 * (Windows 11 shows a restore glyph when maximized).
 */
export function TitleBar() {
	const [maximized, setMaximized] = useState(false)
	const action =
		(run: (win: ReturnType<typeof getCurrentWindow>) => Promise<unknown>) =>
		() => {
			try {
				void run(getCurrentWindow())
			} catch {
				/* not running inside Tauri (e.g. tests) */
			}
		}
	useEffect(() => {
		let unlisten: (() => void) | undefined
		try {
			const win = getCurrentWindow()
			void win.isMaximized().then(setMaximized)
			void win
				.onResized(() => {
					void win.isMaximized().then(setMaximized)
				})
				.then(stop => {
					unlisten = stop
				})
		} catch {
			/* not running inside Tauri (e.g. tests) */
		}
		return () => unlisten?.()
	}, [])
	return (
		<div
			data-tauri-drag-region
			className='app-titlebar flex h-9 shrink-0 items-center justify-between pl-3 pr-0.5 select-none'
		>
			<div
				data-tauri-drag-region
				className='pointer-events-none flex items-center gap-2 text-xs font-medium text-slate-600'
			>
				<img
					src='/app-icon.png'
					alt=''
					className='size-4 rounded-[0.3rem]'
				/>
				<span>Windows Apps</span>
			</div>
			<div className='flex items-center'>
				<button
					type='button'
					aria-label='Minimize'
					onClick={action(win => win.minimize())}
					className='grid h-9 w-11 place-items-center text-slate-500 transition-colors hover:bg-slate-200/70 hover:text-slate-800'
				>
					<Minus size={15} />
				</button>
				<button
					type='button'
					aria-label={maximized ? 'Restore' : 'Maximize'}
					onClick={action(win => win.toggleMaximize())}
					className='grid h-9 w-11 place-items-center text-slate-500 transition-colors hover:bg-slate-200/70 hover:text-slate-800'
				>
					{maximized ? <Copy size={12} /> : <Square size={11} />}
				</button>
				<button
					type='button'
					aria-label='Close'
					onClick={action(win => win.close())}
					className='grid h-9 w-11 place-items-center text-slate-500 transition-colors hover:bg-red-500 hover:text-white'
				>
					<X size={16} />
				</button>
			</div>
		</div>
	)
}
