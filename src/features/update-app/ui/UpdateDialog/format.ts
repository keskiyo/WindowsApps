export function formatBytes(bytes: number): string {
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function formatReleaseDate(value: string): string | null {
	const date = new Date(value)
	if (Number.isNaN(date.getTime())) return null
	return new Intl.DateTimeFormat('en-GB', {
		day: '2-digit',
		month: 'short',
		year: 'numeric',
		timeZone: 'UTC',
	}).format(date)
}
