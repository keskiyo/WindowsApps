import { chooseCustomCategoryAccent } from '../../entities/category'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface CategoryActionOptions {
	set: SetAppState
	get: GetAppState
	persist: PersistPreferences
	idFactory: () => string
}

type CategoryActions = Pick<
	AppState,
	| 'createCategory'
	| 'renameCategory'
	| 'deleteCategory'
	| 'reorderCategory'
	| 'toggleCategory'
>

function nameTaken(
	categories: AppState['categories'],
	value: string,
	exceptId?: string,
): boolean {
	return categories.some(
		category =>
			category.id !== exceptId &&
			category.label.toLocaleLowerCase() === value.toLocaleLowerCase(),
	)
}

/** The user's own category list: creating, renaming, deleting, ordering and collapsing it. */
export function createCategoryActions({
	set,
	get,
	persist,
	idFactory,
}: CategoryActionOptions): CategoryActions {
	return {
		createCategory(label) {
			const value = label.trim()
			if (!value) return { ok: false, error: 'Enter a category name' }
			if (nameTaken(get().categories, value))
				return { ok: false, error: 'Category name already exists' }
			const id = idFactory()
			const accent = chooseCustomCategoryAccent(
				get().categories.flatMap(category =>
					category.builtIn || !category.accent ? [] : [category.accent],
				),
			)
			set(state => ({
				categories: [
					...state.categories,
					{ id, label: value, builtIn: false, accent },
				],
				categoryOrder: [id, ...state.categoryOrder],
			}))
			persist()
			return { ok: true, id }
		},
		renameCategory(id, label) {
			const value = label.trim()
			if (!value) return { ok: false, error: 'Enter a category name' }
			if (nameTaken(get().categories, value, id))
				return { ok: false, error: 'Category name already exists' }
			if (!get().categories.some(category => category.id === id))
				return { ok: false, error: 'Category not found' }
			set(state => ({
				categories: state.categories.map(category =>
					category.id === id ? { ...category, label: value } : category,
				),
			}))
			persist()
			return { ok: true }
		},
		deleteCategory(id) {
			const category = get().categories.find(
				category => category.id === id,
			)
			if (!category || category.builtIn)
				return {
					ok: false,
					error: 'Built-in categories cannot be deleted',
				}
			set(state => ({
				categories: state.categories.filter(
					category => category.id !== id,
				),
				categoryOrder: state.categoryOrder.filter(
					category => category !== id,
				),
				collapsedCategories: state.collapsedCategories.filter(
					category => category !== id,
				),
				categoryOverrides: Object.fromEntries(
					Object.entries(state.categoryOverrides).map(
						([appId, category]) => [
							appId,
							category === id ? 'other' : category,
						],
					),
				),
				categoryOverrideIdentities: Object.fromEntries(
					Object.entries(state.categoryOverrideIdentities).map(
						([identity, category]) => [
							identity,
							category === id ? 'other' : category,
						],
					),
				),
			}))
			persist()
			return { ok: true }
		},
		reorderCategory(active, over) {
			set(state => {
				const from = state.categoryOrder.indexOf(active)
				const to = state.categoryOrder.indexOf(over)
				if (from < 0 || to < 0 || from === to) return state
				const categoryOrder = [...state.categoryOrder]
				categoryOrder.splice(to, 0, categoryOrder.splice(from, 1)[0])
				return { categoryOrder }
			})
			persist()
		},
		toggleCategory(category) {
			set(state => ({
				collapsedCategories: state.collapsedCategories.includes(category)
					? state.collapsedCategories.filter(item => item !== category)
					: [...state.collapsedCategories, category],
			}))
			persist()
		},
	}
}
