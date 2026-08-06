import { INSTALLERS_DOCS_CATEGORY, isCatalogArtifact } from '../../entities/app'
import { addUnique, identityOf } from './reconciliation'
import type { AppState, PersistPreferences, SetAppState } from './types'

interface AppPlacementOptions {
	set: SetAppState
	persist: PersistPreferences
}

/** Which bucket an application is filed under: a category, or the manual installer mark. */
export function createAppPlacementActions({
	set,
	persist,
}: AppPlacementOptions): Pick<AppState, 'moveApp'> {
	return {
		moveApp(id, category) {
			set(state => {
				const app = state.apps.find(item => item.id === id)
				// `state.apps` is the backend snapshot, so this asks the scanner's verdict,
				// not the derived one: an installer or documentation entry the scan itself
				// classified stays in its bucket and cannot be filed by hand.
				if (app && isCatalogArtifact(app)) return state
				const identity = app ? identityOf(app) : id
				// Filing into Installers & Docs always means "this is an installer": the scan
				// detects documentation reliably on its own, so a manual mark has one meaning.
				if (category === INSTALLERS_DOCS_CATEGORY)
					return {
						installerAppIds: addUnique(state.installerAppIds, id),
						installerAppIdentities: addUnique(
							state.installerAppIdentities,
							identity,
						),
						// An artifact cannot be a favorite — `toggleFavorite` refuses one and
						// the card hides the toggle — so the star has to go with the move.
						favoriteAppIds: state.favoriteAppIds.filter(
							appId => appId !== id,
						),
						favoriteAppIdentities: state.favoriteAppIdentities.filter(
							item => item !== identity,
						),
					}
				// Any other destination is also the way back out of Installers & Docs.
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
