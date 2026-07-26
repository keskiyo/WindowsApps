import { describe, expect, it } from 'vitest'
import {
	getDropAction,
	groupAppsByCategory,
	sortFavoritesFirst,
} from '../../../src/lib/catalog'
import type { AppInfo } from '../../../src/types'

const apps = [
	{ id: 'wow', name: 'World of Warcraft', category: 'games' },
	{ id: 'code', name: 'Code', category: 'development' },
] as AppInfo[]

describe('catalog utilities', () => {
	it('groups applications without mutating their order', () => {
		const original = [...apps]
		const groups = groupAppsByCategory(apps)
		expect(groups.get('games')).toEqual([apps[0]])
		expect(groups.get('development')).toEqual([apps[1]])
		expect(apps).toEqual(original)
	})

	it('keeps every member of a category in encounter order', () => {
		const many = [
			{ id: 'a', name: 'A', category: 'games' },
			{ id: 'b', name: 'B', category: 'development' },
			{ id: 'c', name: 'C', category: 'games' },
			{ id: 'd', name: 'D', category: 'games' },
		] as AppInfo[]
		const groups = groupAppsByCategory(many)
		expect(groups.get('games')?.map(app => app.id)).toEqual(['a', 'c', 'd'])
		expect(groups.get('development')?.map(app => app.id)).toEqual(['b'])
	})

	it('puts favorites first, each group sorted alphabetically by name', () => {
		const games = [
			{ id: 'steam', name: 'Steam', category: 'games' },
			{ id: 'telegram', name: 'Telegram', category: 'games' },
			{ id: 'discord', name: 'Discord', category: 'games' },
			{ id: 'battlenet', name: 'Battle.net', category: 'games' },
		] as AppInfo[]
		// Favorites come first sorted alphabetically among themselves, then the rest alphabetically.
		const ordered = sortFavoritesFirst(games, ['telegram', 'battlenet'])
		expect(ordered.map(app => app.id)).toEqual([
			'battlenet',
			'telegram',
			'discord',
			'steam',
		])
		// Original array is not mutated.
		expect(games[0].id).toBe('steam')
	})

	it('routes an application drop to a category', () => {
		expect(
			getDropAction(
				{ type: 'app', appId: 'wow', category: 'other' },
				{ type: 'category', category: 'games' },
			),
		).toEqual({ type: 'move-app', appId: 'wow', category: 'games' })
	})
})
