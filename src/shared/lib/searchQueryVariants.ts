const ENGLISH_KEYS = "`qwertyuiop[]asdfghjkl;'zxcvbnm,."
const RUSSIAN_KEYS = 'ёйцукенгшщзхъфывапролджэячсмитьбю'

function remapToken(token: string, from: string, to: string): string | null {
	let changed = false
	const mapped = [...token]
		.map(character => {
			const index = from.indexOf(character)
			if (index < 0) return character
			changed = true
			return to[index]
		})
		.join('')
	return changed ? mapped : null
}

const TRANSLITERATION: Record<string, string> = {
	а: 'a',
	б: 'b',
	в: 'v',
	г: 'g',
	д: 'd',
	е: 'e',
	ё: 'e',
	ж: 'zh',
	з: 'z',
	и: 'i',
	й: 'y',
	к: 'k',
	л: 'l',
	м: 'm',
	н: 'n',
	о: 'o',
	п: 'p',
	р: 'r',
	с: 's',
	т: 't',
	у: 'u',
	ф: 'f',
	х: 'h',
	ц: 'ts',
	ч: 'ch',
	ш: 'sh',
	щ: 'sch',
	ъ: '',
	ы: 'y',
	ь: '',
	э: 'e',
	ю: 'yu',
	я: 'ya',
}

function transliterate(token: string): string | null {
	let changed = false
	const latin = [...token]
		.map(character => {
			const replacement = TRANSLITERATION[character]
			if (replacement === undefined) return character
			changed = true
			return replacement
		})
		.join('')
	return changed && latin.length > 0 ? latin : null
}

export function queryTokenVariants(token: string): string[] {
	const variants = new Set([token])
	const russian = remapToken(token, ENGLISH_KEYS, RUSSIAN_KEYS)
	const english = remapToken(token, RUSSIAN_KEYS, ENGLISH_KEYS)
	if (russian) variants.add(russian)
	if (english) variants.add(english)
	const latin = transliterate(token)
	if (latin) variants.add(latin)
	return [...variants]
}

export function isWithinOneEdit(left: string, right: string): boolean {
	if (Math.abs(left.length - right.length) > 1) return false
	if (left === right) return true
	if (left.length === right.length) {
		const differences: number[] = []
		for (let index = 0; index < left.length; index += 1)
			if (left[index] !== right[index]) differences.push(index)
		if (differences.length === 1) return true
		return (
			differences.length === 2 &&
			differences[1] === differences[0] + 1 &&
			left[differences[0]] === right[differences[1]] &&
			left[differences[1]] === right[differences[0]]
		)
	}
	const [shorter, longer] =
		left.length < right.length ? [left, right] : [right, left]
	let shortIndex = 0
	let longIndex = 0
	let skipped = false
	while (shortIndex < shorter.length && longIndex < longer.length) {
		if (shorter[shortIndex] === longer[longIndex]) {
			shortIndex += 1
		} else if (skipped) {
			return false
		} else {
			skipped = true
		}
		longIndex += 1
	}
	return true
}
