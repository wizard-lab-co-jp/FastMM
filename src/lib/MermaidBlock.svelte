<script lang="ts">
    import { onMount, tick } from 'svelte';

    export let id: string;
    export let plainText: string;
    export let onSync: (id: string, plainText: string) => void;

    let isEditing = false;
    let mermaidEl: HTMLElement;
    let textareaEl: HTMLTextAreaElement;
    let editContent = "";
    
    let lastValidSvg = "";
    let errorMsg = "";
    
    let mermaidAPI: any = null;

    async function renderMermaid() {
        if (!mermaidAPI) return;
        const code = plainText.replace(/^```mermaid(\r?\n)?/m, '').replace(/(\r?\n)?```$/m, '').trim();
        try {
            const valid = await mermaidAPI.parse(code);
            if (valid) {
                const { svg } = await mermaidAPI.render(`mermaid-${id.replace(/-/g, '')}-${Date.now()}`, code);
                lastValidSvg = svg;
                errorMsg = "";
            }
        } catch (err: any) {
            errorMsg = err.message || "Syntax Error";
        }
        
        if (mermaidEl && lastValidSvg) {
            mermaidEl.innerHTML = lastValidSvg;
        }
    }

    onMount(async () => {
        try {
            const m = await import('mermaid');
            mermaidAPI = m.default;
            mermaidAPI.initialize({ startOnLoad: false, theme: 'dark' });
            renderMermaid();
        } catch (e) {
            console.error('Failed to load mermaid', e);
        }
    });

    $: if (plainText && !isEditing && mermaidAPI) {
        renderMermaid();
    }

    async function toggleEdit() {
        if (!isEditing) {
            editContent = plainText;
            isEditing = true;
            await tick();
            textareaEl?.focus();
        } else {
            handleBlur();
        }
    }

    function handleBlur() {
        if (!isEditing) return;
        isEditing = false;
        if (editContent !== plainText) {
            let newText = editContent.trim();
            if (!newText.startsWith('```mermaid')) newText = `\`\`\`mermaid\n${newText}`;
            if (!newText.endsWith('```')) newText = `${newText}\n\`\`\``;
            onSync(id, newText);
        } else {
            renderMermaid();
        }
    }
</script>

<div class="mermaid-block-wrapper">
    <div class="mermaid-render-area" on:dblclick={toggleEdit}>
        <div class="mermaid-svg-container" bind:this={mermaidEl}>
            {#if !lastValidSvg && !errorMsg}
                <span style="color: #888;">Loading diagram...</span>
            {/if}
        </div>
        {#if errorMsg}
            <div class="error-indicator">Syntax Error: {errorMsg}</div>
        {/if}
    </div>
    
    <div class="edit-toggle">
        <button on:click={toggleEdit}>{isEditing ? 'Close Code' : 'Edit Code'}</button>
    </div>

    {#if isEditing}
        <textarea 
            class="mermaid-editor font-mono" 
            bind:this={textareaEl}
            bind:value={editContent}
            on:blur={handleBlur}
        ></textarea>
    {/if}
</div>

<style>
    .mermaid-block-wrapper {
        margin: 1rem 0;
        position: relative;
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 6px;
        padding: 0.5rem;
    }
    .mermaid-render-area {
        cursor: pointer;
        position: relative;
        display: flex;
        flex-direction: column;
        align-items: center;
        min-height: 50px;
    }
    .mermaid-svg-container {
        width: 100%;
        display: flex;
        justify-content: center;
        overflow-x: auto;
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
    .mermaid-editor {
        width: 100%;
        min-height: 150px;
        background: #1a1a1a;
        color: #e0e0e0;
        border: 1px solid #444;
        border-radius: 4px;
        padding: 0.5rem;
        margin-top: 0.5rem;
        font-family: monospace;
        resize: vertical;
        outline: none;
        box-sizing: border-box;
    }
    .mermaid-editor:focus {
        border-color: #4a90e2;
    }
    .font-mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }
</style>
