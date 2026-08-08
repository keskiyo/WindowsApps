const FIELD_LABELS: Record<string, string> = {
	name: 'display name',
	product: 'product name',
	exe: 'executable',
	path: 'install path',
	description: 'file description',
	publisher: 'publisher',
	feature: 'Windows feature marker',
}

const STANDALONE_LABELS: Record<string, string> = {
	'source=steam': 'Installed through Steam',
	'rule=wsl-start-app': 'A WSL distribution',
	'default=no-signal': 'No signal strong enough to classify',
}

export function categoryReasonsLabel(reasons: string[] | undefined) {
	if (!reasons?.length) return null
	const parts = reasons.flatMap(reason => {
		const standalone = STANDALONE_LABELS[reason]
		if (standalone) return [standalone]
		const separator = reason.indexOf('=')
		if (separator < 0) return []
		const field = FIELD_LABELS[reason.slice(0, separator)]
		const needle = reason.slice(separator + 1)
		return field && needle ? [`${field} “${needle}”`] : []
	})
	return parts.length > 0 ? parts.join(', ') : null
}
