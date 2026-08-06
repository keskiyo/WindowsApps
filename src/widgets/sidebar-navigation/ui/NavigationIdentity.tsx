import type { NavigationIdentityProps } from '../types'

export function NavigationIdentity({ onGoHome }: NavigationIdentityProps) {
	return (
		<button
			type="button"
			aria-label="Go to All Apps"
			onClick={onGoHome}
			className="flex w-full min-w-0 items-center gap-3 rounded-xl text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
		>
			<img
				src="/app-icon.png"
				alt=""
				className="size-10 shrink-0 rounded-xl object-cover ring-1 ring-violet-400/25 ring-inset"
			/>
			<span className="min-w-0 truncate text-[1.05rem] font-semibold tracking-tight text-(--text-primary)">
				Windows Apps
			</span>
		</button>
	)
}
