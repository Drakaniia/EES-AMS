<script lang="ts">
	import { settingsState } from './settings-state.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import logo from '$lib/assets/logo-seal.png';
	import {
		pickBrandingLogo,
		resetBranding,
		deleteBrandingLogo
	} from '$lib/features/settings/native';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';

	let saving = $state(false);
	let titleInput = $state('');
	let charCount = $derived(titleInput.length);

	// Sync local title state from settings
	$effect(() => {
		if (settingsStore.settings) {
			titleInput = settingsStore.settings.brandingTitle ?? 'EES AMS';
		}
	});

	const activeLogo = $derived.by(() => {
		const path = settingsStore.settings?.brandingLogoPath;
		if (path) {
			return convertFileSrc(path);
		}
		return logo;
	});

	const MAX_TITLE_LENGTH = 40;

	// Preset logos (using the default seal + lucide-style icons as placeholders)
	const presets: { name: string; label: string }[] = [
		{ name: 'seal', label: 'School Seal' },
		{ name: 'graduation', label: 'Graduation' },
		{ name: 'book', label: 'Book' },
		{ name: 'apple', label: 'Apple' }
	];

	async function handleUpload() {
		saving = true;
		try {
			await pickBrandingLogo();
			await settingsStore.load();
			const s = settingsStore.settings;
			if (s) {
				settingsState.brandingLogoPath = s.brandingLogoPath ?? null;
				settingsState.brandingTitle = s.brandingTitle ?? 'EES AMS';
			}
			settingsState.toast('Logo uploaded');
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Upload failed';
			if (msg !== 'User cancelled file picker') {
				settingsState.toast(`Upload failed: ${msg}`, false);
			}
		} finally {
			saving = false;
		}
	}

	async function handleReset() {
		saving = true;
		try {
			await resetBranding();
			await settingsStore.load();
			const s = settingsStore.settings;
			if (s) {
				titleInput = s.brandingTitle ?? 'EES AMS';
				settingsState.brandingLogoPath = s.brandingLogoPath ?? null;
				settingsState.brandingTitle = s.brandingTitle ?? 'EES AMS';
			}
			settingsState.toast('Branding reset to defaults');
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Reset failed';
			settingsState.toast(`Reset failed: ${msg}`, false);
		} finally {
			saving = false;
		}
	}

	function handleTitleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		titleInput = target.value.slice(0, MAX_TITLE_LENGTH);
		settingsState.brandingTitle = titleInput || 'EES AMS';
	}

	function handleTitleBlur() {
		const trimmed = titleInput.trim();
		if (!trimmed) {
			titleInput = 'EES AMS';
			settingsState.brandingTitle = 'EES AMS';
		}
	}
</script>

<div class="space-y-5 rounded-2xl border border-border bg-card p-6">
	<div class="space-y-1">
		<h3 class="text-lg font-medium">Sidebar Branding</h3>
		<p class="text-xs text-muted-foreground">
			Customize the logo and title shown in the sidebar header.
		</p>
	</div>

	<!-- Logo Preview -->
	<div class="space-y-3">
		<span class="label-mono text-sm">Logo Image</span>
		<div
			class="flex flex-col items-center gap-4 rounded-xl border border-dashed border-border bg-surface p-6"
		>
			<div
				class="flex size-24 items-center justify-center overflow-hidden rounded-2xl bg-background ring-1 ring-border"
			>
				<img
					src={activeLogo}
					alt="Current branding logo"
					class="size-full object-contain p-1"
				/>
			</div>

			<div class="flex flex-wrap items-center justify-center gap-2">
				<button
					type="button"
					onclick={handleUpload}
					disabled={saving}
					class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if saving}
						<Spinner />
					{/if}
					Upload Image
				</button>
				<button
					type="button"
					onclick={handleReset}
					disabled={saving}
					class="rounded-pill border border-border px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-60"
				>
					Reset to Default
				</button>
			</div>
		</div>
	</div>

	<!-- Preset Gallery -->
	<div class="space-y-3">
		<span class="label-mono text-sm">Preset Gallery</span>
		<div class="grid grid-cols-4 gap-2">
			{#each presets as preset}
				<button
					type="button"
					class="flex flex-col items-center gap-1.5 rounded-xl border border-border bg-surface p-3 text-xs text-muted-foreground transition-colors hover:border-primary/50 hover:bg-background hover:text-foreground"
					title={preset.label}
				>
					<span class="grid size-10 place-items-center rounded-lg bg-background ring-1 ring-border">
						<!-- Preset icon placeholder -->
						<span class="text-lg">{preset.name === 'seal' ? '🏛️' : preset.name === 'graduation' ? '🎓' : preset.name === 'book' ? '📖' : '🍎'}</span>
					</span>
					<span class="truncate w-full text-center">{preset.label}</span>
				</button>
			{/each}
		</div>
	</div>

	<!-- Title Input -->
	<div class="space-y-2">
		<label for="branding-title" class="label-mono text-sm">Title Text</label>
		<div class="relative">
			<input
				id="branding-title"
				type="text"
				value={titleInput}
				oninput={handleTitleInput}
				onblur={handleTitleBlur}
				maxlength={MAX_TITLE_LENGTH}
				placeholder="EES AMS"
				class="h-10 w-full rounded-md border border-border bg-background px-3 pr-16 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
			<span
				class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground tabular-nums"
			>
				{charCount}/{MAX_TITLE_LENGTH}
			</span>
		</div>
	</div>

	<!-- Tip -->
	<div
		class="rounded-xl border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground"
	>
		<span class="font-medium text-foreground">💡 Tip:</span>
		Enter a title like "Mrs. Santos" or "Room 201 - Grade 3" to personalize your sidebar.
	</div>
</div>
