import type { CustomCategoryAccent } from '../model/category.types'

export const CUSTOM_CATEGORY_ACCENTS = [
	'yellow',
	'cyan',
	'pink',
	'green',
	'blue',
	'orange',
	'purple',
	'red',
] as const satisfies readonly CustomCategoryAccent[]

export function isCustomCategoryAccent(
	value: unknown,
): value is CustomCategoryAccent {
	return (
		typeof value === 'string' &&
		CUSTOM_CATEGORY_ACCENTS.includes(value as CustomCategoryAccent)
	)
}

export function stableCustomCategoryAccent(
	id: string,
): CustomCategoryAccent {
	let hash = 0
	for (const character of id)
		hash = (hash * 31 + character.charCodeAt(0)) | 0
	return CUSTOM_CATEGORY_ACCENTS[Math.abs(hash) % CUSTOM_CATEGORY_ACCENTS.length]!
}

export function chooseCustomCategoryAccent(
	used: Iterable<CustomCategoryAccent>,
	random = Math.random,
): CustomCategoryAccent {
	const occupied = new Set(used)
	const available = CUSTOM_CATEGORY_ACCENTS.filter(
		accent => !occupied.has(accent),
	)
	const palette = available.length ? available : CUSTOM_CATEGORY_ACCENTS
	return palette[Math.floor(random() * palette.length)]!
}
