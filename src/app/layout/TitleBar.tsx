import { Copy, Minus, Square, X } from 'lucide-react'
import { useWindowControls } from '../../shared/platform/window/useWindowControls'

export function TitleBar() {
	const controls = useWindowControls()
	return (
		<div
			data-tauri-drag-region
			className="app-titlebar flex h-9 shrink-0 items-center justify-between pr-0.5 pl-3 select-none"
		>
			<div
				data-tauri-drag-region
				className="pointer-events-none flex items-center gap-2 text-xs font-medium text-slate-600"
			>
				<img
					src="/app-icon.png"
					alt=""
					className="size-4 rounded-[0.3rem]"
				/>
				<span>Windows Apps</span>
			</div>
			<div className="flex items-center">
				<button
					type="button"
					aria-label="Minimize"
					onClick={controls.minimize}
					className="grid h-9 w-11 place-items-center text-slate-400 transition-colors hover:bg-slate-700/70 hover:text-white"
				>
					<Minus size={15} />
				</button>
				<button
					type="button"
					aria-label={controls.maximized ? 'Restore' : 'Maximize'}
					onClick={controls.toggleMaximize}
					className="grid h-9 w-11 place-items-center text-slate-400 transition-colors hover:bg-slate-700/70 hover:text-white"
				>
					{controls.maximized ? (
						<Copy size={12} />
					) : (
						<Square size={11} />
					)}
				</button>
				<button
					type="button"
					aria-label="Close"
					onClick={controls.close}
					className="grid h-9 w-11 place-items-center text-slate-500 transition-colors hover:bg-red-500 hover:text-white"
				>
					<X size={16} />
				</button>
			</div>
		</div>
	)
}
