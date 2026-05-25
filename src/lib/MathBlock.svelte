<script lang="ts">
    import katex from 'katex';
    import { onMount, tick } from 'svelte';

    export let id: string;
    export let plainText: string;
    export let onSync: (id: string, plainText: string) => void;

    let isEditing = false;
    let mathEl: HTMLElement;
    let textareaEl: HTMLTextAreaElement;
    let editContent = "";

    function renderMath() {
        if (!mathEl) return;
        try {
            const mathContent = plainText.replace(/^\$\$(\r?\n)?/m, '').replace(/(\r?\n)?\$\$$/m, '').trim();
            katex.render(mathContent, mathEl, { displayMode: true, throwOnError: false });
        } catch (e) {
            console.error(e);
        }
    }

    onMount(() => {
        renderMath();
    });

    $: if (plainText && !isEditing) {
        renderMath();
    }

    async function toggleEdit() {
        if (!isEditing) {
            editContent = plainText;
            isEditing = true;
            await tick();
            textareaEl?.focus();
        }
    }

    function handleBlur() {
        isEditing = false;
        if (editContent !== plainText) {
            let newText = editContent.trim();
            if (!newText.startsWith('$$')) newText = `$$\n${newText}`;
            if (!newText.endsWith('$$')) newText = `${newText}\n$$`;
            onSync(id, newText);
        } else {
            renderMath();
        }
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            handleBlur();
        }
    }
</script>

<div class="math-block-wrapper" on:dblclick={toggleEdit}>
    {#if isEditing}
        <textarea 
            class="math-editor font-mono" 
            bind:this={textareaEl}
            bind:value={editContent}
            on:blur={handleBlur}
            on:keydown={handleKeyDown}
        ></textarea>
    {:else}
        <div class="math-render" bind:this={mathEl}></div>
    {/if}
</div>

<style>
    .math-block-wrapper {
        margin: 1rem 0;
        cursor: pointer;
        position: relative;
    }
    .math-editor {
        width: 100%;
        min-height: 100px;
        background: #2a2a2a;
        color: #e0e0e0;
        border: 1px solid #555;
        border-radius: 4px;
        padding: 0.5rem;
        font-family: monospace;
        resize: vertical;
        outline: none;
        box-sizing: border-box;
    }
    .math-editor:focus {
        border-color: #4a90e2;
    }
    .math-render {
        padding: 0.5rem;
        border: 1px solid transparent;
        transition: border-color 0.2s;
        border-radius: 4px;
        background: rgba(255, 255, 255, 0.02);
    }
    .math-block-wrapper:hover .math-render {
        border-color: rgba(255, 255, 255, 0.1);
    }
    .font-mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }
</style>
