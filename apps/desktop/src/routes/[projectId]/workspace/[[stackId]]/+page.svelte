<script lang="ts">
	import StackTabs from '$components/StackTabs.svelte';
	import WorktreeChanges from '$components/WorktreeChanges.svelte';
	import { SettingsService } from '$lib/config/appSettingsV2';
	import { getContext } from '@gitbutler/shared/context';
	import type { PageData } from './$types';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	const settingsService = getContext(SettingsService);
	const settingsStore = settingsService.appSettings;

	const { data }: { data: PageData } = $props();

	const projectId = $derived(data.projectId);
	const stackId = $derived(page.params.stackId);

	// Redirect to board if we have switched away from V3 feature.
	$effect(() => {
		if ($settingsStore && !$settingsStore.featureFlags.v3) {
			goto(`/${data.projectId}/board`);
		}
	});
</script>

<div class="workspace">
	<div class="left">
		<WorktreeChanges {projectId} />
	</div>
	<div class="right">
		<StackTabs {projectId} selectedId={stackId} />
		<div class="branch"></div>
	</div>
</div>

<style>
	.workspace {
		display: flex;
		flex: 1;
		align-items: stretch;
		padding-bottom: 16px;
		padding-right: 16px;
		height: 100%;
		gap: 14px;
	}

	.left {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		width: 290px;
		background-color: var(--clr-bg-1);
		border-radius: var(--radius-ml);
		border: 1px solid var(--clr-border-2);
	}

	.right {
		display: flex;
		flex: 1;
		flex-direction: column;
	}

	.branch {
		border: 1px solid var(--clr-border-2);
		flex: 1;
		border-radius: 0 var(--radius-ml) var(--radius-ml);
	}
</style>
