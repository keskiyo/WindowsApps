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
		moveApp(id, category, artifact = 'installer') {
			set(state => {
				const app = state.apps.find(item => item.id === id)
				if (app && isCatalogArtifact(app)) return state
				const identity = app ? identityOf(app) : id
				const withoutInstaller = {
					installerAppIds: state.installerAppIds.filter(
						appId => appId !== id,
					),
					installerAppIdentities: state.installerAppIdentities.filter(
						item => item !== identity,
					),
				}
				const withoutDocument = {
					documentAppIds: state.documentAppIds.filter(
						appId => appId !== id,
					),
					documentAppIdentities: state.documentAppIdentities.filter(
						item => item !== identity,
					),
				}
				if (category === INSTALLERS_DOCS_CATEGORY)
					return {
						...withoutInstaller,
						...withoutDocument,
						...(artifact === 'documentation'
							? {
									documentAppIds: addUnique(
										withoutDocument.documentAppIds,
										id,
									),
									documentAppIdentities: addUnique(
										withoutDocument.documentAppIdentities,
										identity,
									),
								}
							: {
									installerAppIds: addUnique(
										withoutInstaller.installerAppIds,
										id,
									),
									installerAppIdentities: addUnique(
										withoutInstaller.installerAppIdentities,
										identity,
									),
								}),
						favoriteAppIds: state.favoriteAppIds.filter(
							appId => appId !== id,
						),
						favoriteAppIdentities:
							state.favoriteAppIdentities.filter(
								item => item !== identity,
							),
					}
				return {
					...withoutInstaller,
					...withoutDocument,
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
