import { INSTALLERS_DOCS_CATEGORY, isCatalogArtifact } from '../../entities/app'
import { addUnique, identityOf } from './reconciliation'
import type { AppState, PersistPreferences, SetAppState } from './types'

interface AppPlacementOptions {
	set: SetAppState
	persist: PersistPreferences
}

export function createAppPlacementActions({
	set,
	persist,
}: AppPlacementOptions): Pick<AppState, 'moveApp'> {
	return {
		moveApp(id, category) {
			set(state => {
				const app = state.apps.find(item => item.id === id)
				if (app && isCatalogArtifact(app)) return state
				const identity = app ? identityOf(app) : id
				if (category === INSTALLERS_DOCS_CATEGORY)
					return {
						installerAppIds: addUnique(state.installerAppIds, id),
						installerAppIdentities: addUnique(
							state.installerAppIdentities,
							identity,
						),
						favoriteAppIds: state.favoriteAppIds.filter(
							appId => appId !== id,
						),
						favoriteAppIdentities:
							state.favoriteAppIdentities.filter(
								item => item !== identity,
							),
					}
				return {
					installerAppIds: state.installerAppIds.filter(
						appId => appId !== id,
					),
					installerAppIdentities: state.installerAppIdentities.filter(
						item => item !== identity,
					),
					categoryOverrides: {
						...state.categoryOverrides,
						[id]: category,
					},
					categoryOverrideIdentities: {
						...state.categoryOverrideIdentities,
						[identity]: category,
					},
				}
			})
			persist()
		},
	}
}
