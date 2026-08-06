import { isCatalogArtifact } from '../../entities/app'
import { addUnique, identityOf } from './reconciliation'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface AppMarkOptions {
	set: SetAppState
	get: GetAppState
	persist: PersistPreferences
}

type AppMarkActions = Pick<
	AppState,
	| 'toggleFavorite'
	| 'hideApp'
	| 'restoreApp'
	| 'promoteAuxiliary'
	| 'demoteAuxiliary'
>

/** What the user marked about one application: favorite, hidden, promoted out of Auxiliary. */
export function createAppMarkActions({
	set,
	get,
	persist,
}: AppMarkOptions): AppMarkActions {
	return {
		toggleFavorite(id) {
			set(state => {
				const app = state.apps.find(item => item.id === id)
				if (app && isCatalogArtifact(app)) return state
				const promoted = app
					? state.promotedAppIds.includes(app.id) ||
						state.promotedAppIdentities.includes(identityOf(app))
					: false
				if (app?.visibilityClass === 'auxiliary' && !promoted) return state
				const identity = app ? identityOf(app) : id
				const wasFavorite = state.favoriteAppIds.includes(id)
				return {
					favoriteAppIds: wasFavorite
						? state.favoriteAppIds.filter(appId => appId !== id)
						: [...state.favoriteAppIds, id],
					favoriteAppIdentities: wasFavorite
						? state.favoriteAppIdentities.filter(
								item => item !== identity,
							)
						: addUnique(state.favoriteAppIdentities, identity),
				}
			})
			persist()
		},
		hideApp(id) {
			set(state => {
				if (state.hiddenAppIds.includes(id)) return state
				const app = state.apps.find(item => item.id === id)
				const identity = app ? identityOf(app) : id
				return {
					hiddenAppIds: [...state.hiddenAppIds, id],
					hiddenAppIdentities: addUnique(
						state.hiddenAppIdentities,
						identity,
					),
				}
			})
			persist()
		},
		restoreApp(id) {
			set(state => {
				const app = state.apps.find(item => item.id === id)
				const identity = app ? identityOf(app) : id
				return {
					hiddenAppIds: state.hiddenAppIds.filter(
						appId => appId !== id,
					),
					hiddenAppIdentities: state.hiddenAppIdentities.filter(
						item => item !== identity,
					),
				}
			})
			persist()
		},
		promoteAuxiliary(id) {
			const app = get().apps.find(item => item.id === id)
			const identity = app ? identityOf(app) : id
			set(state => ({
				promotedAppIdentities: state.promotedAppIdentities.includes(
					identity,
				)
					? state.promotedAppIdentities
					: [...state.promotedAppIdentities, identity],
			}))
			persist()
		},
		demoteAuxiliary(id) {
			const app = get().apps.find(item => item.id === id)
			const identity = app ? identityOf(app) : id
			set(state => ({
				promotedAppIds: state.promotedAppIds.filter(
					appId => appId !== id,
				),
				promotedAppIdentities: state.promotedAppIdentities.filter(
					item => item !== identity,
				),
				favoriteAppIds: state.favoriteAppIds.filter(
					appId => appId !== id,
				),
				favoriteAppIdentities: state.favoriteAppIdentities.filter(
					item => item !== identity,
				),
			}))
			persist()
		},
	}
}
