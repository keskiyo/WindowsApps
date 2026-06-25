import { invoke } from '@tauri-apps/api/core'
import type { VpnClient } from '../types'

export const tauriVpnClient: VpnClient = {
	list: () => invoke('vpn_list'),
	set: (id, enabled) => invoke('vpn_set', { id, enabled }),
	setup: id => invoke('vpn_setup', { id }),
}
