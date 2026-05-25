<script lang="ts">
    import { onMount, tick } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';

    export let id: string;
    export let plainText: string;
    export let onSync: (id: string, plainText: string) => void;

    type TypstCompileResult =
        | { type: 'success'; svg: string }
        | { type: 'error'; message: string };

    let isEditing = false;
    let svgContent = '';
    let errorMsg = '';
    let editContent = '';
    let textareaEl: HTMLTextAreaElement;

    function extractCode(text: string): string {
        return text
            .replace(/^```typst(\r?\n)?/m, '')
            .replace(/(\r?\n)?```$/m, '')
            .trim();
    }

    function wrapCode(code: string): string {
        let t = code.trim();
        if (!t.startsWith('```typst')) t = `\`\`\`typst\n${t}`;
        if (!t.endsWith('```')) t = `${t}\n\`\`\``;
        return t;
    }

    async function renderTypst() {
        const code = extractCode(plainText);
        if (!code) return;
        try {
            const result = await invoke<TypstCompileResult>('compile_typst', { source: code });
            if (result.type === 'success') {
                svgContent = result.svg;
                errorMsg = '';
            } else {
                errorMsg = result.message;
            }
        } catch (err: any) {
            errorMsg = String(err);
        }
    }

    // Initial render and reactive re-render when plainText changes outside edit mode
    onMount(() => { renderTypst(); });
    $: if (plainText && !isEditing) { renderTypst(); }

    async function toggleEdit() {
        if (!isEditing) {
            editContent = plainText;
            isEditing = true;
            await tick();
            textareaEl?.focus();
        } else {
            await handleBlur();
        }
    }

    async function handleBlur() {
        if (!isEditing) return;
        isEditing = false;
        const newText = wrapCode(editContent);
        if (newText !== plainText) {
            onSync(id, newText);
        } else {
            await renderTypst();
        }
    }

    async function handleKeyDown(e: KeyboardEvent) {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
            e.preventDefault();
            await handleBlur();
        }
    }
</script>

<div class="typst-block-wrapper">
    <div class="typst-render-area" on:dblclick={toggleEdit} role="button" tabindex="-1">
        {#if svgContent && !errorMsg}
            {@html svgContent}
        {:else if !svgContent && !errorMsg}
            <span class="placeholder">Loading Typst…</span>
        {/if}
        {#if errorMsg}
            <div class="error-indicator">Compile Error: {errorMsg}</div>
        {/if}
    </div>

    <div class="edit-toggle">
        <button on:click={toggleEdit}>{isEditing ? 'Close Code' : 'Edit Code'}</button>
    </div>

    {#if isEditing}
        <textarea
            class="typst-editor font-mono"
            bind:this={textareaEl}
            bind:value={editContent}
            on:blur={handleBlur}
            on:keydown={handleKeyDown}
        ></textarea>
    {/if}
</div>

<style>
    .typst-block-wrapper {
        margin: 1rem 0;
        position: relative;
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 6px;
        padding: 0.5rem;
    }
    .typst-render-area {
        cursor: pointer;
        display: flex;
        flex-direction: column;
        align-items: center;
        min-height: 50px;
        overflow-x: auto;
    }
    .placeholder {
        color: #888;
        font-size: 0.85rem;
    }
    .error-indicator {
        color: #ef4444;
        font-size: 0.75rem;
        margin-top: 0.5rem;
        background: rgba(239, 68, 68, 0.1);
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        align-self: center;
        max-width: 100%;
        white-space: pre-wrap;
    }
    .edit-toggle {
        display: flex;
        justify-content: flex-end;
        margin-top: 0.5rem;
    }
    .edit-toggle button {
        background: transparent;
        border: 1px solid #555;
        color: #aaa;
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        font-size: 0.8rem;
        cursor: pointer;
    }
    .edit-toggle button:hover {
        background: #333;
        color: white;
    }
    .typst-editor {
        width: 100%;
        min-height: 150px;
        background: #1a1a1a;
        color: #e0e0e0;
        border: 1px solid #444;
        border-radius: 4px;
        padding: 0.5rem;
        margin-top: 0.5rem;
        resize: vertical;
        outline: none;
        box-sizing: border-box;
    }
    .typst-editor:focus {
        border-color: #4a90e2;
    }
    .font-mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }
</style>
