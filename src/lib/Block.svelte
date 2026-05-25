<script lang="ts">
    import type { InlineDecorationNode } from './domUtils';
    import InlineDecoration from './InlineDecoration.svelte';
    
    export let id: string;
    export let blockType: any;
    export let astContent: InlineDecorationNode[];
    export let onInput: (id: string, element: HTMLElement) => void;
    export let onKeyDown: (id: string, e: KeyboardEvent, element: HTMLElement) => void;
    export let onMove: (nodeId: string, targetPreviousSiblingId: string | null) => void;
    export let onFormat: (id: string, actionType: string, element: HTMLElement, metaValue?: string) => void = () => {};
    export let onHistory: (type: 'undo' | 'redo') => void = () => {};
  
    let blockEl: HTMLElement;
    let isComposing = false;
    let isDragOver = false;
    let showLinkDialog = false;
    let linkUrl = "";

    function handleInput() {
        if (!isComposing) {
            onInput(id, blockEl);
        }
    }

    function handleCompositionStart() {
        isComposing = true;
    }

    function handleCompositionEnd() {
        isComposing = false;
        onInput(id, blockEl);
    }

    function submitLink() {
        showLinkDialog = false;
        onFormat(id, 'link', blockEl, linkUrl);
        linkUrl = "";
    }

    function handleKeyDownEvent(e: KeyboardEvent) {
        if (isComposing) return;

        const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
        const mod = isMac ? e.metaKey : e.ctrlKey;

        if (mod && e.key === 'b') { e.preventDefault(); onFormat(id, 'bold', blockEl); return; }
        if (mod && e.key === 'i') { e.preventDefault(); onFormat(id, 'italic', blockEl); return; }
        if (mod && e.key === 'e') { e.preventDefault(); onFormat(id, 'code', blockEl); return; }
        if (mod && e.key === 'k') { e.preventDefault(); showLinkDialog = true; return; }
        if (mod && e.key === 'z' && !e.shiftKey) { e.preventDefault(); onHistory('undo'); return; }
        if (mod && (e.key === 'y' || (e.shiftKey && e.key === 'z'))) { e.preventDefault(); onHistory('redo'); return; }

        if (mod && e.altKey && e.key >= '1' && e.key <= '6') {
            e.preventDefault();
            onFormat(id, 'heading', blockEl, e.key);
            return;
        }

        if (mod && e.shiftKey && e.key === '8') {
            e.preventDefault();
            onFormat(id, 'list', blockEl);
            return;
        }

        if (e.key === 'Tab') {
            e.preventDefault();
        }
        onKeyDown(id, e, blockEl);
    }

    function handleDragStart(e: DragEvent) {
        if (e.dataTransfer) {
            e.dataTransfer.setData('text/plain', id);
            e.dataTransfer.effectAllowed = 'move';
        }
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = 'move';
        }
        isDragOver = true;
    }

    function handleDragLeave() {
        isDragOver = false;
    }

    function handleDrop(e: DragEvent) {
        e.preventDefault();
        isDragOver = false;
        const draggedId = e.dataTransfer?.getData('text/plain');
        if (draggedId && draggedId !== id) {
            onMove(draggedId, id);
        }
    }

    $: isHeading = blockType && blockType.heading;
    $: headingLevel = isHeading ? blockType.heading.level : 0;
    $: isList = blockType && blockType.list;
    $: listIndent = isList ? blockType.list.indentLevel : 0;
</script>
  
<div class="block-wrapper" 
     class:drag-over={isDragOver}
     on:dragover={handleDragOver}
     on:dragleave={handleDragLeave}
     on:drop={handleDrop}>
    
    <div class="drag-handle" draggable="true" on:dragstart={handleDragStart}>
        &#8942;&#8942;
    </div>

    {#if isHeading}
        <svelte:element this={"h" + headingLevel}
            class="fastmm-block heading-{headingLevel}"
            contenteditable="true"
            bind:this={blockEl}
            on:input={handleInput}
            on:compositionstart={handleCompositionStart}
            on:compositionend={handleCompositionEnd}
            on:keydown={handleKeyDownEvent}
            data-block-id={id}
        >
            {#each astContent as node (node.key)}
                <InlineDecoration {node} />
            {/each}
        </svelte:element>
    {:else}
        <div
            class="fastmm-block"
            class:list={isList}
            style={isList ? `margin-left: ${listIndent * 2}rem;` : ''}
            contenteditable="true"
            bind:this={blockEl}
            on:input={handleInput}
            on:compositionstart={handleCompositionStart}
            on:compositionend={handleCompositionEnd}
            on:keydown={handleKeyDownEvent}
            data-block-id={id}
        >
            {#if isList}
                <span class="list-bullet">•</span>
            {/if}
            {#each astContent as node (node.key)}
                <InlineDecoration {node} />
            {/each}
        </div>
    {/if}

    {#if showLinkDialog}
        <div class="link-dialog">
            <input type="url" placeholder="URL を入力..." bind:value={linkUrl}
                   on:keydown|stopPropagation={(e) => {
                       if (e.key === 'Enter') { submitLink(); }
                       if (e.key === 'Escape') { showLinkDialog = false; }
                   }} />
        </div>
    {/if}
</div>
  
<style>
    .block-wrapper {
        display: flex;
        align-items: flex-start;
        position: relative;
        margin-bottom: 0.2rem;
        border: 2px solid transparent;
        transition: border-color 0.2s;
    }
    .block-wrapper.drag-over {
        border-top-color: #4a90e2;
    }

    .drag-handle {
        width: 24px;
        cursor: grab;
        color: #555;
        font-size: 1.2rem;
        line-height: 1.5;
        user-select: none;
        opacity: 0;
        transition: opacity 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
        padding-top: 0.2rem;
    }
    .block-wrapper:hover .drag-handle {
        opacity: 1;
    }
    .drag-handle:active {
        cursor: grabbing;
    }

    .fastmm-block {
        flex: 1;
        min-height: 1.5em;
        padding: 0.25rem 0.5rem;
        outline: none;
        word-break: break-word;
        white-space: pre-wrap;
        position: relative;
    }
    
    .fastmm-block:focus {
        background-color: rgba(255, 255, 255, 0.03);
        border-radius: 4px;
    }

    .heading-1 { font-size: 2.2em; font-weight: 800; margin-top: 1.2rem; color: #fff; }
    .heading-2 { font-size: 1.8em; font-weight: 700; margin-top: 1rem; color: #f0f0f0; }
    .heading-3 { font-size: 1.5em; font-weight: 700; margin-top: 0.8rem; color: #e0e0e0; }
    .heading-4 { font-size: 1.25em; font-weight: 600; margin-top: 0.6rem; color: #d0d0d0; }
    .heading-5 { font-size: 1.1em; font-weight: 600; margin-top: 0.4rem; color: #c0c0c0; }
    .heading-6 { font-size: 1em; font-weight: 600; margin-top: 0.3rem; color: #a0a0a0; }
    
    .list { display: flex; }
    .list-bullet { margin-right: 0.5rem; color: #888; user-select: none; }

    .link-dialog {
        position: absolute;
        top: 100%;
        left: 2rem;
        z-index: 100;
        background: #333;
        padding: 0.5rem;
        border-radius: 4px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    }
    .link-dialog input {
        background: #222;
        color: white;
        border: 1px solid #555;
        padding: 0.3rem 0.5rem;
        border-radius: 4px;
        width: 250px;
        outline: none;
    }
    .link-dialog input:focus {
        border-color: #4a90e2;
    }
</style>
