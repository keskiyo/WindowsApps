export async function copyToClipboard(value: string): Promise<void> {
	if (navigator.clipboard?.writeText) {
		await navigator.clipboard.writeText(value)
		return
	}
	const element = document.createElement('textarea')
	element.value = value
	element.setAttribute('readonly', '')
	element.className = 'fixed -left-full top-0 opacity-0'
	document.body.append(element)
	element.select()
	const copied = document.execCommand?.('copy')
	element.remove()
	if (!copied) throw new Error('Clipboard is unavailable')
}
