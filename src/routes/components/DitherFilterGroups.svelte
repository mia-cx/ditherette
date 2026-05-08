<script lang="ts">
	import { Checkbox } from '$lib/components/ui/checkbox';
	import type { DitherField, DitherMethod } from './dither-options';

	type Props = {
		methodFilters: DitherMethod[];
		fieldFilters: DitherField[];
		onToggleMethod: (method: DitherMethod) => void;
		onToggleField: (field: DitherField) => void;
		compact?: boolean;
	};

	let {
		methodFilters,
		fieldFilters,
		onToggleMethod,
		onToggleField,
		compact = false
	}: Props = $props();

	const groupClass = $derived(compact ? 'grid gap-1.5' : 'grid gap-2');
	const labelClass = $derived(
		compact
			? 'flex items-center gap-1.5 text-xs text-muted-foreground'
			: 'flex items-center gap-2 text-xs text-muted-foreground'
	);
</script>

<div class={groupClass}>
	<span class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase"
		>Method</span
	>
	<label class={labelClass}>
		<Checkbox
			checked={methodFilters.includes('threshold')}
			onCheckedChange={() => onToggleMethod('threshold')}
			aria-label="Toggle threshold method filter"
		/>
		Threshold
	</label>
	<label class={labelClass}>
		<Checkbox
			checked={methodFilters.includes('error-diffusion')}
			onCheckedChange={() => onToggleMethod('error-diffusion')}
			aria-label="Toggle error-diffusion method filter"
		/>
		Error diffusion
	</label>
	<label class={labelClass}>
		<Checkbox
			checked={methodFilters.includes('none')}
			onCheckedChange={() => onToggleMethod('none')}
			aria-label="Toggle none method filter"
		/>
		None
	</label>
</div>
<div class={groupClass}>
	<span class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase">Field</span
	>
	<label class={labelClass}>
		<Checkbox
			checked={fieldFilters.includes('ordered')}
			onCheckedChange={() => onToggleField('ordered')}
			aria-label="Toggle ordered field filter"
		/>
		Ordered
	</label>
	<label class={labelClass}>
		<Checkbox
			checked={fieldFilters.includes('noise')}
			onCheckedChange={() => onToggleField('noise')}
			aria-label="Toggle noise field filter"
		/>
		Noise
	</label>
	<label class={labelClass}>
		<Checkbox
			checked={fieldFilters.includes('kernel')}
			onCheckedChange={() => onToggleField('kernel')}
			aria-label="Toggle kernel field filter"
		/>
		Kernel
	</label>
	<label class={labelClass}>
		<Checkbox
			checked={fieldFilters.includes('none')}
			onCheckedChange={() => onToggleField('none')}
			aria-label="Toggle none field filter"
		/>
		None
	</label>
</div>
