interface Props {
	onGoHome(): void
}

/**
 * The product identity at the top of the sidebar and of the drawer. It doubles as the way home:
 * the same control used to sit in the app header, and moving it here left nothing else that
 * returns to All Apps *and* scrolls the catalog back to the top.
 */
export function NavigationIdentity({ onGoHome }: Props) {
	return (
		<button
			type='button'
			aria-label='Go to All Apps'
			onClick={onGoHome}
			className='flex w-full min-w-0 items-center gap-3 rounded-xl text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)'
		>
			<img
				src='/app-icon.png'
				alt=''
				className='size-10 shrink-0 rounded-xl object-cover ring-1 ring-inset ring-violet-400/25'
			/>
			<span className='min-w-0 truncate text-[1.05rem] font-semibold tracking-tight text-(--text-primary)'>
				Windows Apps
			</span>
		</button>
	)
}
