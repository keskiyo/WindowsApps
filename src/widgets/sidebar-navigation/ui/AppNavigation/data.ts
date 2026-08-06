export function navigationItemClass(active: boolean) {
	return `relative flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-sm transition-[background-color,border-color,box-shadow] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none ${active ? 'border-(--accent) bg-(--utility-accent) font-medium text-(--text-primary) shadow-(--shadow-active-navigation)' : 'border-(--border-neutral) bg-(--surface-panel) text-(--text-primary) hover:bg-(--surface-raised)'}`
}
