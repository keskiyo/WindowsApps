import type { SettingsSectionHeaderProps } from '../../types'

export function SettingsSectionHeader({
	icon: Icon,
	title,
	description,
}: SettingsSectionHeaderProps) {
	return (
		<>
			<span className="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner">
				<Icon size={19} aria-hidden="true" />
			</span>
			<div className="min-w-0 flex-1">
				<h2 className="font-medium">{title}</h2>
				<p className="mt-1 text-sm leading-6 text-slate-600">
					{description}
				</p>
			</div>
		</>
	)
}
