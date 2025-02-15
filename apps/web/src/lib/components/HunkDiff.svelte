<script lang="ts">
	import { CountColumnSide, SectionType, type ContentSection, type Line } from '$lib/diffParsing';
	import { create } from '@gitbutler/shared/codeHighlight';
	import diff_match_patch from 'diff-match-patch';

	interface Props {
		filePath: string;
		subsections: ContentSection[];
		tabSize?: number;
		wrapText?: boolean;
		diffFont?: string;
		diffLigatures?: boolean;
		inlineUnifiedDiffs?: boolean;
		diffContrast?: 'light' | 'medium' | 'strong';
	}

	const {
		filePath,
		subsections,
		tabSize = 4,
		wrapText = true,
		diffFont,
		diffLigatures = true,
		diffContrast = 'medium',
		inlineUnifiedDiffs = false
	}: Props = $props();

	const WHITESPACE_REGEX = /\s/;
	const BORDER_WIDTH = 1;

	let tableWidth = $state<number>(0);
	let tableHeight = $state<number>(0);
	let numberHeaderWidth = $state<number>(0);

	function charDiff(text1: string, text2: string): { 0: number; 1: string }[] {
		const differ = new diff_match_patch();
		const diff = differ.diff_main(text1, text2);
		differ.diff_cleanupSemantic(diff);
		return diff;
	}

	function isLineEmpty(lines: Line[]) {
		const whitespaceRegex = new RegExp(WHITESPACE_REGEX);
		if (!lines[0]?.content.match(whitespaceRegex)) {
			return true;
		}

		return false;
	}

	function createRowData(section: ContentSection): Row[] {
		return section.lines.map((line) => {
			// if (line.content === '') {
			// 	// Add extra \n for empty lines for correct copy/pasting output
			// 	line.content = '\n';
			// }

			return {
				beforeLineNumber: line.beforeLineNumber,
				afterLineNumber: line.afterLineNumber,
				tokens: toTokens(line.content),
				type: section.sectionType,
				size: line.content.length,
				isLast: false
			};
		});
	}

	function sanitize(text: string) {
		const element = document.createElement('div');
		element.innerText = text;
		return element.innerHTML;
	}

	function toTokens(inputLine: string): string[] {
		let highlighter = create(inputLine, filePath);
		let tokens: string[] = [];
		highlighter.highlight((text, classNames) => {
			const token = classNames
				? `<span data-no-drag class=${classNames}>${sanitize(text)}</span>`
				: sanitize(text);

			tokens.push(token);
		});

		return tokens;
	}

	function computeWordDiff(prevSection: ContentSection, nextSection: ContentSection): DiffRows {
		const numberOfLines = nextSection.lines.length;
		const returnRows: DiffRows = {
			prevRows: [],
			nextRows: []
		};

		// Loop through every line in the section
		// We're only bothered with prev/next sections with equal # of lines changes
		for (let i = 0; i < numberOfLines; i++) {
			const oldLine = prevSection.lines[i] as Line;
			const newLine = nextSection.lines[i] as Line;
			const prevSectionRow = {
				beforeLineNumber: oldLine.beforeLineNumber,
				afterLineNumber: oldLine.afterLineNumber,
				tokens: [] as string[],
				type: prevSection.sectionType,
				size: oldLine.content.length,
				isLast: false
			};
			const nextSectionRow = {
				beforeLineNumber: newLine.beforeLineNumber,
				afterLineNumber: newLine.afterLineNumber,
				tokens: [] as string[],
				type: nextSection.sectionType,
				size: newLine.content.length,
				isLast: false
			};

			const diff = charDiff(oldLine.content, newLine.content);

			for (const token of diff) {
				const text = token[1];
				const type = token[0];

				if (type === Operation.Equal) {
					prevSectionRow.tokens.push(...toTokens(text));
					nextSectionRow.tokens.push(...toTokens(text));
				} else if (type === Operation.Insert) {
					nextSectionRow.tokens.push(
						`<span data-no-drag class="token-inserted">${sanitize(text)}</span>`
					);
				} else if (type === Operation.Delete) {
					prevSectionRow.tokens.push(
						`<span data-no-drag class="token-deleted">${sanitize(text)}</span>`
					);
				}
			}
			returnRows.nextRows.push(nextSectionRow);
			returnRows.prevRows.push(prevSectionRow);
		}

		return returnRows;
	}

	function computeInlineWordDiff(prevSection: ContentSection, nextSection: ContentSection): Row[] {
		const numberOfLines = nextSection.lines.length;

		const rows = [];

		// Loop through every line in the section
		// We're only bothered with prev/next sections with equal # of lines changes
		for (let i = 0; i < numberOfLines; i++) {
			const oldLine = prevSection.lines[i] as Line;
			const newLine = nextSection.lines[i] as Line;

			const sectionRow = {
				beforeLineNumber: newLine.beforeLineNumber,
				afterLineNumber: newLine.afterLineNumber,
				tokens: [] as string[],
				type: nextSection.sectionType,
				size: newLine.content.length,
				isLast: false
			};

			const diff = charDiff(oldLine.content, newLine.content);

			for (const token of diff) {
				const text = token[1];
				const type = token[0];

				if (type === Operation.Equal) {
					sectionRow.tokens.push(...toTokens(text));
				} else if (type === Operation.Insert) {
					sectionRow.tokens.push(
						`<span data-no-drag class="token-inserted">${sanitize(text)}</span>`
					);
				} else if (type === Operation.Delete) {
					sectionRow.tokens.push(
						`<span data-no-drag class="token-deleted token-strikethrough">${sanitize(text)}</span>`
					);
				}
			}
			rows.push(sectionRow);
		}

		return rows;
	}

	function generateRows(subsections: ContentSection[]) {
		const rows = subsections.reduce((acc, nextSection, i) => {
			const prevSection = subsections[i - 1];

			// Filter out section for which we don't need to compute word diffs
			if (!prevSection || nextSection.sectionType === SectionType.Context) {
				acc.push(...createRowData(nextSection));
				return acc;
			}

			if (prevSection.sectionType === SectionType.Context) {
				acc.push(...createRowData(nextSection));
				return acc;
			}

			if (prevSection.lines.length !== nextSection.lines.length) {
				acc.push(...createRowData(nextSection));
				return acc;
			}

			if (isLineEmpty(prevSection.lines)) {
				acc.push(...createRowData(nextSection));
				return acc;
			}

			if (inlineUnifiedDiffs) {
				const rows = computeInlineWordDiff(prevSection, nextSection);

				acc.splice(-prevSection.lines.length);

				acc.push(...rows);
				return acc;
			} else {
				const { prevRows, nextRows } = computeWordDiff(prevSection, nextSection);

				// Insert returned row datastructures into the correct place
				// Find and replace previous rows with tokenized version
				prevRows.forEach((row, previousRowIndex) => {
					acc[acc.length - (prevRows.length - previousRowIndex)] = row;
				});

				acc.push(...nextRows);

				return acc;
			}
		}, [] as Row[]);

		const last = rows.at(-1);
		if (last) {
			last.isLast = true;
		}

		return rows;
	}

	const renderRows = $derived(generateRows(subsections));

	interface DiffHunkLineInfo {
		beforLineStart: number;
		beforeLineCount: number;
		afterLineStart: number;
		afterLineCount: number;
	}

	function getHunkLineInfo(subsections: ContentSection[]): DiffHunkLineInfo {
		const firstSection = subsections[0];
		const lastSection = subsections.at(-1);

		const beforLineStart = firstSection?.lines[0]?.beforeLineNumber ?? 0;
		const beforeLineEnd = lastSection?.lines?.at(-1)?.beforeLineNumber ?? 0;
		const beforeLineCount = beforeLineEnd - beforLineStart + 1;

		const afterLineStart = firstSection?.lines[0]?.afterLineNumber ?? 0;
		const afterLineEnd = lastSection?.lines?.at(-1)?.afterLineNumber ?? 0;
		const afterLineCount = afterLineEnd - afterLineStart + 1;

		return {
			beforLineStart,
			beforeLineCount,
			afterLineStart,
			afterLineCount
		};
	}

	const hunkLineInfo = $derived(getHunkLineInfo(subsections));

	interface Row {
		beforeLineNumber?: number;
		afterLineNumber?: number;
		tokens: string[];
		type: SectionType;
		size: number;
		isLast: boolean;
	}

	enum Operation {
		Equal = 0,
		Insert = 1,
		Delete = -1,
		Edit = 2
	}

	type DiffRows = { prevRows: Row[]; nextRows: Row[] };
</script>

{#snippet countColumn(row: Row, side: CountColumnSide)}
	<td
		class="table__numberColumn"
		data-no-drag
		class:diff-line-deletion={row.type === SectionType.RemovedLines}
		class:diff-line-addition={row.type === SectionType.AddedLines}
		align="center"
		class:is-last={row.isLast}
		class:is-before={side === CountColumnSide.Before}
	>
		{side === CountColumnSide.Before ? row.beforeLineNumber : row.afterLineNumber}
	</td>
{/snippet}

<div
	bind:clientWidth={tableWidth}
	bind:clientHeight={tableHeight}
	class="table__wrapper hide-native-scrollbar contrast-{diffContrast}"
	style="--tab-size: {tabSize}; --diff-font: {diffFont};"
	style:font-variant-ligatures={diffLigatures ? 'common-ligatures' : 'none'}
>
	<table class="table__section">
		<thead class="table__title">
			<tr>
				<th
					bind:clientWidth={numberHeaderWidth}
					class="table__checkbox-container"
					style="--border-width: {BORDER_WIDTH}px;"
					colspan={2}
				>
					<div
						class="table__title-content"
						style="--number-col-width: {numberHeaderWidth}px; --table-width: {tableWidth}px; --border-width: {BORDER_WIDTH}px; --top: -{BORDER_WIDTH}px"
					>
						<span>
							{`@@ -${hunkLineInfo.beforLineStart},${hunkLineInfo.beforeLineCount} +${hunkLineInfo.afterLineStart},${hunkLineInfo.afterLineCount} @@`}
						</span>
					</div>
				</th>
			</tr>
		</thead>

		<tbody>
			{#each renderRows as row}
				<tr data-no-drag>
					{@render countColumn(row, CountColumnSide.Before)}
					{@render countColumn(row, CountColumnSide.After)}
					<td
						class="table__textContent"
						style="--tab-size: {tabSize}; --wrap: {wrapText ? 'wrap' : 'nowrap'}"
						class:readonly={true}
						data-no-drag
						class:diff-line-deletion={row.type === SectionType.RemovedLines}
						class:diff-line-addition={row.type === SectionType.AddedLines}
						class:is-last={row.isLast}
					>
						{@html row.tokens.join('')}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style lang="postcss">
	.table__wrapper {
		border-radius: var(--radius-s);
		background-color: var(--clr-diff-line-bg);
		border: 1px solid var(--clr-border-2);
		overflow-x: auto;
		width: 100%;
	}

	table,
	.table__section {
		width: 100%;
		font-family: var(--diff-font);
		border-collapse: separate;
		border-spacing: 0;
	}

	thead {
		width: 100%;
		padding: 0;
	}

	tbody {
		z-index: var(--z-lifted);
	}

	th,
	td,
	tr {
		padding: 0;
		margin: 0;
	}

	table thead th {
		top: 0;
		left: 0;
		position: sticky;
		height: 28px;
	}

	.table__checkbox-container {
		z-index: var(--z-lifted);

		border-right: 1px solid var(--clr-border-2);
		border-bottom: 1px solid var(--clr-border-2);
		background-color: var(--clr-diff-count-bg);
		border-top-left-radius: var(--radius-s);
		box-sizing: border-box;

		&.selected {
			background-color: var(--clr-diff-selected-count-bg);
			border-color: var(--clr-diff-selected-count-border);
		}
	}

	.table__checkbox {
		padding: 4px 6px;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.table__title {
		user-select: none;
	}

	.draggable {
		cursor: grab;
	}

	.table__drag-handle {
		position: fixed;
		right: 6px;
		top: 6px;
		box-sizing: border-box;
		background-color: var(--clr-bg-1);
		display: flex;
		justify-content: center;
		align-items: center;
		border-radius: var(--radius-s);
		opacity: 0;
		transform: scale(0.9);
		transform-origin: top right;
		pointer-events: none;
		color: var(--clr-text-2);
		transition:
			opacity 0.2s,
			transform 0.2s;
	}

	.table__lock {
		position: fixed;
		right: 6px;
		top: 6px;
		box-sizing: border-box;
		background-color: var(--clr-theme-warn-soft);
		display: flex;
		justify-content: center;
		align-items: center;
		border-radius: var(--radius-s);
		pointer-events: none;
		color: var(--clr-text-2);
		transition: transform var(--transition-medium);
	}

	.table__title-content {
		color: var(--clr-text-2, #867e79);
		font-family: 'Geist Mono';
		font-size: 12px;
		font-style: normal;
		font-weight: 400;
		line-height: 120%; /* 14.4px */

		position: absolute;
		top: var(--top);
		left: var(--number-col-width);
		width: calc(var(--table-width) - var(--number-col-width));
		height: calc(100% + var(--border-width) * 2);
		box-sizing: border-box;
		padding: 4px 6px;
		text-wrap: nowrap;

		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--clr-border-2);
		border-top-right-radius: var(--radius-m);
	}

	.table__numberColumn {
		color: var(--clr-diff-count-text, #b4afac);
		font-family: 'Geist Mono';
		font-size: 11px;
		font-style: normal;
		font-weight: 400;
		line-height: 120%; /* 13.2px */

		border-color: var(--clr-diff-count-border);
		background-color: var(--clr-diff-count-bg);
		padding: 0 4px;
		text-align: right;
		vertical-align: top;
		user-select: none;

		box-sizing: border-box;
		position: sticky;
		left: calc(var(--number-col-width));
		width: var(--number-col-width);
		min-width: var(--number-col-width);

		&.diff-line-addition {
			background-color: var(--clr-diff-addition-count-bg);
			color: var(--clr-diff-addition-count-text);
			border-color: var(--clr-diff-addition-count-border);
		}

		&.diff-line-deletion {
			background-color: var(--clr-diff-deletion-count-bg);
			color: var(--clr-diff-deletion-count-text);
			border-color: var(--clr-diff-deletion-count-border);
		}

		&.selected {
			background-color: var(--clr-diff-selected-count-bg);
			color: var(--clr-diff-selected-count-text);
			border-color: var(--clr-diff-selected-count-border);
		}

		&.is-before.is-last {
			border-bottom-left-radius: var(--radius-s);
		}
	}

	.table__numberColumn:first-of-type {
		width: var(--number-col-width);
		min-width: var(--number-col-width);
		left: 0px;
	}

	.table__textContent {
		z-index: var(--z-lifted);
		width: 100%;

		color: var(--clr-text-1, #1a1614);
		font-family: 'Geist Mono';
		font-size: 12px;
		font-style: normal;
		font-weight: 400;
		line-height: 120%; /* 14.4px */

		padding-left: 4px;
		tab-size: var(--tab-size);
		white-space: pre;
		user-select: text;
		cursor: text;
		text-wrap: var(--wrap);
		border-left: 1px solid var(--clr-border-2);
	}

	/* DIFF LINE */
	.diff-line-marker-addition,
	.diff-line-addition {
		background-color: var(--clr-diff-addition-line-bg);
	}

	.diff-line-marker-deletion,
	.diff-line-deletion {
		background-color: var(--clr-diff-deletion-line-bg);
	}

	/* CONTRAST MODIFIERS */
	.table__wrapper {
		&.contrast-light {
			--clr-diff-count-text: var('--', var(--clr-diff-count-text));
			/* deletion */
			--clr-diff-deletion-line-bg: var('--', var(--clr-diff-deletion-line-bg));
			--clr-diff-deletion-line-highlight: var('--', var(--clr-diff-deletion-line-highlight));
			--clr-diff-deletion-count-bg: var('--', var(--clr-diff-deletion-count-bg));
			--clr-diff-deletion-count-text: var('--', var(--clr-diff-deletion-count-text));
			--clr-diff-deletion-count-border: var('--', var(--clr-diff-deletion-count-border));
			/* addition */
			--ctx-diff-addition-line-bg: var('--', var(--clr-diff-addition-line-bg));
			--clr-diff-addition-line-highlight: var('--', var(--clr-diff-addition-line-highlight));
			--clr-diff-addition-count-bg: var('--', var(--clr-diff-addition-count-bg));
			--clr-diff-addition-count-text: var('--', var(--clr-diff-addition-count-text));
			--clr-diff-addition-count-border: var('--', var(--clr-diff-addition-count-border));
		}

		&.contrast-medium {
			--clr-diff-count-text: var(--clr-diff-count-text-contrast-2);
			/* deletion */
			--clr-diff-deletion-line-bg: var(--clr-diff-deletion-contrast-2-line-bg);
			--clr-diff-deletion-line-highlight: var(--clr-diff-deletion-contrast-2-line-highlight);
			--clr-diff-deletion-count-bg: var(--clr-diff-deletion-contrast-2-count-bg);
			--clr-diff-deletion-count-text: var(--clr-diff-deletion-contrast-2-count-text);
			--clr-diff-deletion-count-border: var(--clr-diff-deletion-contrast-2-count-border);
			/* addition */
			--clr-diff-addition-line-bg: var(--clr-diff-addition-contrast-2-line-bg);
			--clr-diff-addition-line-highlight: var(--clr-diff-addition-contrast-2-line-highlight);
			--clr-diff-addition-count-bg: var(--clr-diff-addition-contrast-2-count-bg);
			--clr-diff-addition-count-text: var(--clr-diff-addition-contrast-2-count-text);
			--clr-diff-addition-count-border: var(--clr-diff-addition-contrast-2-count-border);
		}

		&.contrast-strong {
			--clr-diff-count-text: var(--clr-diff-count-text-contrast-3);
			/* deletion */
			--clr-diff-deletion-line-bg: var(--clr-diff-deletion-contrast-3-line-bg);
			--clr-diff-deletion-line-highlight: var(--clr-diff-deletion-contrast-3-line-highlight);
			--clr-diff-deletion-count-bg: var(--clr-diff-deletion-contrast-3-count-bg);
			--clr-diff-deletion-count-text: var(--clr-diff-deletion-contrast-3-count-text);
			--clr-diff-deletion-count-border: var(--clr-diff-deletion-contrast-3-count-border);
			/* addition */
			--clr-diff-addition-line-bg: var(--clr-diff-addition-contrast-3-line-bg);
			--clr-diff-addition-line-highlight: var(--clr-diff-addition-contrast-3-line-highlight);
			--clr-diff-addition-count-bg: var(--clr-diff-addition-contrast-3-count-bg);
			--clr-diff-addition-count-text: var(--clr-diff-addition-contrast-3-count-text);
			--clr-diff-addition-count-border: var(--clr-diff-addition-contrast-3-count-border);
		}
	}
</style>
