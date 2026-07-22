import { useEffect, useRef, useState } from 'react'
import { toAppClientError } from '../lib/tauri'
import type { ScanSettings, SystemClient, SystemSettings } from '../types'

interface Options {
	client: SystemClient
	onForceFullScan?: () => Promise<void>
	onResetCatalogCache?: () => Promise<void>
}

type PathKind = 'includedPaths' | 'excludedPaths'

/**
 * Owns all SettingsPage state and side effects (load, autostart, scan settings, scan
 * paths, catalog maintenance) so the component stays presentational.
 */
export function useSystemSettings({
	client,
	onForceFullScan,
	onResetCatalogCache,
}: Options) {
	const [settings, setSettings] = useState<SystemSettings | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [saving, setSaving] = useState(false)
	const [confirmForce, setConfirmForce] = useState(false)
	const [forcing, setForcing] = useState(false)
	const [confirmReset, setConfirmReset] = useState(false)
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
				if (active) setError(toAppClientError(reason).message)
			})
		return () => {
			active = false
		}
	}, [client])

	async function toggleAutostart() {
		if (!settings || saving) return
		const enabled = !settings.autostartEnabled
		setSaving(true)
		try {
			await client.setAutostart(enabled)
			setSettings({ ...settings, autostartEnabled: enabled })
		} catch (reason) {
			setError(toAppClientError(reason).message)
		} finally {
			setSaving(false)
		}
	}

	async function saveScanSettings(next: ScanSettings) {
		if (!settings || saving) return
		setSaving(true)
		setError(null)
		try {
			const scanSettings = await client.setScanSettings(next)
			setSettings({ ...settings, scanSettings })
		} catch (reason) {
			setError(toAppClientError(reason).message)
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
		setError(null)
		try {
			await onForceFullScan()
			setConfirmForce(false)
		} catch (reason) {
			setError(toAppClientError(reason).message)
		} finally {
			maintenanceInFlight.current = false
			setForcing(false)
		}
	}

	async function resetCatalogCache() {
		if (!onResetCatalogCache || maintenanceInFlight.current) return
		maintenanceInFlight.current = true
		setResetting(true)
		setError(null)
		try {
			await onResetCatalogCache()
			setConfirmReset(false)
		} catch (reason) {
			setError(toAppClientError(reason).message)
		} finally {
			maintenanceInFlight.current = false
			setResetting(false)
		}
	}

	return {
		settings,
		error,
		saving,
		confirmForce,
		setConfirmForce,
		forcing,
		confirmReset,
		setConfirmReset,
		resetting,
		toggleAutostart,
		saveScanSettings,
		addPath,
		removePath,
		forceFullScan,
		resetCatalogCache,
	}
}
