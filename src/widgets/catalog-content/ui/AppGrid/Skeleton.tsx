export function Skeleton() {
	return (
		<div className="app-card-tile animate-pulse rounded-[1.15rem] border border-white/80 bg-white/48 p-4 shadow-(--shadow-skeleton)">
			<div className="mx-auto mt-3 size-13 rounded-xl bg-slate-300/70" />
			<div className="mx-auto mt-4 h-3 w-2/3 rounded-full bg-slate-300/70" />
		</div>
	)
}
