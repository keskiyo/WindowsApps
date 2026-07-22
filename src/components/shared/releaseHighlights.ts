function cleanMarkdownLine(value: string): string {
	return value
		.replace(/^\s*[-*]\s+/, '')
		.replace(/\[([^\]]+)]\([^)]+\)/g, '$1')
		.replace(/[`*_>#]/g, '')
		.replace(/\s+/g, ' ')
		.trim()
}

function truncateLine(value: string): string {
	return value.length > 180 ? `${value.slice(0, 177).trim()}...` : value
}

export function releaseHighlights(notes: string | null): string[] {
	if (!notes) return []

	const lines = notes.replace(/\r\n/g, '\n').split('\n')
	const highlightHeadingIndex = lines.findIndex(line =>
		/^#{0,6}\s*highlights\s*$/i.test(line.trim()),
	)
	const releaseSectionLines =
		highlightHeadingIndex >= 0 ? lines.slice(highlightHeadingIndex + 1) : lines
	const nextHeadingIndex = releaseSectionLines.findIndex(line =>
		/^#{1,6}\s+\S/.test(line.trim()),
	)
	const searchLines =
		highlightHeadingIndex >= 0 && nextHeadingIndex >= 0
			? releaseSectionLines.slice(0, nextHeadingIndex)
			: releaseSectionLines
	const bullets = searchLines
		.filter(line => /^\s*[-*]\s+/.test(line))
		.map(line => truncateLine(cleanMarkdownLine(line)))
		.filter(Boolean)

	if (bullets.length) return bullets.slice(0, 4)

	return searchLines
		.map(cleanMarkdownLine)
		.filter(Boolean)
		.slice(0, 3)
}
