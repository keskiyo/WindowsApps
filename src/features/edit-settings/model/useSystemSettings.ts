import { useEffect, useRef, useState } from 'react'
import { toAppClientError } from '../../../shared/api/tauri/errors'
import type {
	ScanSettings,
	SystemClient,
	SystemSettings,
} from '../../../entities/system'

interface Options {
	client: SystemClient
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
}

type PathKind = 'includedPaths' | 'excludedPaths'

export type MaintenanceConfirmation = 'force' | 'reset' | null

export type SettingsArea = 'settings' | 'startup' | 'discovery' | 'maintenance'

export function useSystemSettings({
	client,
	onForceFullScan,
	onResetCatalogCache,
}: Options) {
	const [settings, setSettings] = useState<SystemSettings | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [errorArea, setErrorArea] = useState<SettingsArea | null>(null)

	function reportError(area: SettingsArea, message: string) {
		setError(message)
		setErrorArea(area)
	}

	function clearError() {
		setError(null)
		setErrorArea(null)
	}
	const [saving, setSaving] = useState(false)
	const [confirming, setConfirming] = useState<MaintenanceConfirmation>(null)
	const [forcing, setForcing] = useState(false)
	const [resetting, setResetting] = useState(false)
	const maintenanceInFlight = useRef(false)

	useEffect(() => {
		let active = true
		client
			.getSettings()
			.then(value => {
				if (active) setSettings(value)
			})
			.catch(reason => {
				if (active)
					reportError('settings', toAppClientError(reason).message)
			})
		return () => {
			active = false
		}
	}, [client])

	async function toggleAutostart() {
		if (!settings || saving) return
		const enabled = !settings.autostartEnabled
		setSaving(true)
		clearError()
		try {
			await client.setAutostart(enabled)
			setSettings({ ...settings, autostartEnabled: enabled })
		} catch (reason) {
			reportError('startup', toAppClientError(reason).message)
		} finally {
			setSaving(false)
		}
	}

	async function saveScanSettings(next: ScanSettings) {
		if (!settings || saving) return
		setSaving(true)
		clearError()
		try {
			const scanSettings = await client.setScanSettings(next)
			setSettings({ ...settings, scanSettings })
		} catch (reason) {
			reportError('discovery', toAppClientError(reason).message)
		} finally {
			setSaving(false)
		}
	}

	function addPath(kind: PathKind, value: string) {
		const trimmed = value.trim()
		if (!settings || !trimmed) return
		if (
			settings.scanSettings[kind].some(
				path => path.toLowerCase() === trimmed.toLowerCase(),
			)
		)
			return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: [...settings.scanSettings[kind], trimmed],
		})
	}

	function removePath(kind: PathKind, value: string) {
		if (!settings) return
		void saveScanSettings({
			...settings.scanSettings,
			[kind]: settings.scanSettings[kind].filter(path => path !== value),
		})
	}

	async function forceFullScan() {
		if (!onForceFullScan || maintenanceInFlight.current) return
		maintenanceInFlight.current = true
		setForcing(true)
		clearError()
		try {
			await onForceFullScan()
			setConfirming(null)
		} catch (reason) {
			const clientError = toAppClientError(reason)
			if (clientError.code !== 'SCAN_CANCELLED')
				reportError('maintenance', clientError.message)
		} finally {
			maintenanceInFlight.current = false
			setForcing(false)
		}
	}

	async function resetCatalogCache() {
		if (!onResetCatalogCache || maintenanceInFlight.current) return
		maintenanceInFlight.current = true
		setResetting(true)
		clearError()
		try {
			await onResetCatalogCache()
			setConfirming(null)
		} catch (reason) {
			const clientError = toAppClientError(reason)
			if (clientError.code !== 'SCAN_CANCELLED')
				reportError('maintenance', clientError.message)
		} finally {
			maintenanceInFlight.current = false
			setResetting(false)
		}
	}

	return {
		settings,
		error,
		errorArea,
		saving,
		confirming,
		setConfirming,
		forcing,
		resetting,
		toggleAutostart,
		saveScanSettings,
		addPath,
		removePath,
		forceFullScan,
		resetCatalogCache,
	}
}
