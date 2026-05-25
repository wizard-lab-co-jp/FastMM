<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import { extractDecorationsFromDOM, getCaretOffset, getSelectionRange, setCaretOffset } from './domUtils';
    import type { InlineDecorationNode } from './domUtils';
    import Block from './Block.svelte';
    import MathBlock from './MathBlock.svelte';
    import MermaidBlock from './MermaidBlock.svelte';
    import TypstBlock from './TypstBlock.svelte';

    type BlockType = 'paragraph' | 'mathBlock' | 'mermaid' | 'typst' | { heading: { level: number } } | { codeBlock: { language: string } } | { list: { listType: string, indentLevel: number, parentListId: string | null } } | 'blockQuote';

    type BlockData = {
        id: string;
        blockType: BlockType;
        astContent: InlineDecorationNode[];
        plainText: string;
        seq: number;
    };
  
    type VersionEntry = { versionId: string; source: string; label: string };

    let blocks: Record<string, BlockData> = {};
    let nodeOrder: string[] = [];
    let isDirty = false;
    let autoSaveTimeout: ReturnType<typeof setTimeout>;

    // Time Machine panel state
    let showTimeMachine = false;
    let versionHistory: VersionEntry[] = [];

    let visibleBlocks = new Set<string>();
    let observer: IntersectionObserver;

    onMount(() => {
        observer = new IntersectionObserver((entries) => {
            let changed = false;
            entries.forEach(entry => {
                const id = entry.target.getAttribute('data-block-id');
                if (!id) return;
                if (entry.isIntersecting) {
                    if (!visibleBlocks.has(id)) {
                        visibleBlocks.add(id);
                        changed = true;
                    }
                } else {
                    if (visibleBlocks.has(id)) {
                        visibleBlocks.delete(id);
                        changed = true;
                    }
                }
            });
            if (changed) visibleBlocks = visibleBlocks;
        }, { rootMargin: '200px' });

        return () => observer.disconnect();
    });

    function registerBlock(el: HTMLElement) {
        if (observer && el) {
            observer.observe(el);
            visibleBlocks.add(el.getAttribute('data-block-id') as string);
            visibleBlocks = visibleBlocks;
        }
    }

    async function openFile() {
        try {
            const resp: any = await invoke('open_file');
            if (resp) {
                blocks = {};
                for (const b of resp.blocks) {
                    b.seq = 0;
                    blocks[b.id] = b;
                }
                nodeOrder = resp.nodeOrder;
                isDirty = false;
            }
        } catch (err) {
            console.error('Failed to open file:', err);
        }
    }

    function triggerAutoSave() {
        if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
        isDirty = true;
        autoSaveTimeout = setTimeout(async () => {
            if (isDirty) {
                try {
                    // auto_save_silent writes to app_data_dir (cloud-safe)
                    await invoke('auto_save_silent');
                } catch (err) {
                    console.error('Failed to auto-save:', err);
                }
            }
        }, 3000);
    }
  
    async function handleInput(id: string, element: HTMLElement) {
        const block = blocks[id];
        if (!block) return;
  
        const plainText = element.textContent || '';
        const decorations = extractDecorationsFromDOM(element);
        const caretOffset = getCaretOffset(element);
  
        const nextSeq = block.seq + 1;
        block.seq = nextSeq;
        block.astContent = decorations;
        blocks = blocks;
  
        triggerAutoSave();

        try {
            await invoke('sync_block', {
                req: {
                    seq: nextSeq,
                    nodeId: id,
                    plainText,
                    decorations,
                    caretOffset
                }
            });
        } catch (err) {
            console.error('Failed to sync block:', err);
        }
    }
  
    function handleKeyDown(id: string, e: KeyboardEvent, element: HTMLElement) {
        // Additional editor-level logic
    }

    async function handleMoveBlock(nodeId: string, targetPreviousSiblingId: string | null) {
        const currentIndex = nodeOrder.indexOf(nodeId);
        if (currentIndex === -1) return;
        
        nodeOrder.splice(currentIndex, 1);
        
        if (targetPreviousSiblingId) {
            const targetIndex = nodeOrder.indexOf(targetPreviousSiblingId);
            nodeOrder.splice(targetIndex + 1, 0, nodeId);
        } else {
            nodeOrder.unshift(nodeId);
        }
        nodeOrder = nodeOrder;
        
        triggerAutoSave();

        try {
            const resp: any = await invoke('move_block', {
                req: {
                    seq: Date.now(),
                    nodeId,
                    targetParentId: null,
                    targetPreviousSiblingId
                }
            });
            if (resp && resp.success) {
                nodeOrder = resp.newNodeOrder;
            }
        } catch (err) {
            console.error('Failed to move block:', err);
        }
    }

    async function handleGraphicalSync(id: string, plainText: string) {
        const block = blocks[id];
        if (!block) return;
        const nextSeq = block.seq + 1;
        block.seq = nextSeq;
        blocks[id].plainText = plainText;
        blocks = blocks;
        triggerAutoSave();
        try {
            await invoke('sync_block', {
                req: { seq: nextSeq, nodeId: id, plainText, decorations: [], caretOffset: 0 }
            });
        } catch (err) {
            console.error('sync_block failed for graphical block:', err);
        }
    }

    async function handleFormat(id: string, actionType: string, element: HTMLElement, metaValue?: string) {
        const { start, end } = getSelectionRange(element);
        triggerAutoSave();
        try {
            const resp: any = await invoke('apply_format', {
                req: { seq: Date.now(), nodeId: id, actionType, selectionStart: start, selectionEnd: end, metaValue: metaValue ?? null }
            });
            if (resp) {
                blocks[resp.nodeId].astContent = resp.astContent;
                blocks[resp.nodeId].blockType = resp.blockType;
                blocks = blocks;
                
                setTimeout(() => {
                    const el = document.querySelector(`[data-block-id="${resp.nodeId}"]`) as HTMLElement;
                    if (el) setCaretOffset(el, resp.caret.offset);
                }, 0);
            }
        } catch (err) { console.error('apply_format failed:', err); }
    }

    async function handleHistory(type: 'undo' | 'redo') {
        try {
            const resp: any = await invoke('trigger_history', {
                req: { seq: Date.now(), type }
            });
            if (resp) {
                nodeOrder = resp.nodeOrder;
                let newBlocks: Record<string, BlockData> = {};
                for (const rb of resp.restoredBlocks) {
                    newBlocks[rb.id] = {
                        id: rb.id,
                        blockType: rb.blockType,
                        astContent: rb.astContent,
                        plainText: rb.markdown || "",
                        seq: 0
                    };
                }
                blocks = newBlocks;
                isDirty = true;

                setTimeout(() => {
                    const el = document.querySelector(`[data-block-id="${resp.caret.targetNodeId}"]`) as HTMLElement;
                    if (el) setCaretOffset(el, resp.caret.offset);
                }, 0);
            }
        } catch (err) { console.error('trigger_history failed:', err); }
    }

    // ─── Time Machine ────────────────────────────────────────────────────────

    /** Restore a version from Time Machine. Uses handleRestore instead of
     *  handleHistory so that orphaned old block IDs are fully cleaned up. */
    function applyRestoredBlocks(resp: any) {
        // Delete all existing blocks first to avoid orphaned IDs
        for (const id of Object.keys(blocks)) {
            delete blocks[id];
        }
        for (const rb of resp.restoredBlocks) {
            blocks[rb.id] = {
                id: rb.id,
                blockType: rb.blockType,
                astContent: rb.astContent,
                plainText: rb.markdown || '',
                seq: 0,
            };
        }
        nodeOrder = resp.nodeOrder;
        blocks = { ...blocks };
        isDirty = true;
        setTimeout(() => {
            const el = document.querySelector(`[data-block-id="${resp.caret.targetNodeId}"]`) as HTMLElement;
            if (el) setCaretOffset(el, resp.caret.offset);
        }, 0);
    }

    async function openTimeMachine() {
        showTimeMachine = true;
        try {
            const resp: any = await invoke('get_version_history');
            versionHistory = resp.entries ?? [];
        } catch (err) {
            console.error('get_version_history failed:', err);
            versionHistory = [];
        }
    }

    async function restoreVersion(entry: VersionEntry) {
        if (isDirty) {
            const ok = confirm('未保存の変更があります。このバージョンに復元すると失われます。続けますか？');
            if (!ok) return;
        }
        try {
            const resp: any = await invoke('restore_version', {
                req: { seq: Date.now(), versionId: entry.versionId, source: entry.source }
            });
            if (resp) {
                applyRestoredBlocks(resp);
                showTimeMachine = false;
            }
        } catch (err) {
            console.error('restore_version failed:', err);
            alert('復元に失敗しました: ' + String(err));
        }
    }
</script>
  
<div class="toolbar">
    <button on:click={openFile}>Open Markdown File</button>
    <button on:click={() => invoke('save_file')} title="Save (Ctrl+S)">Save</button>
    <button class="history-btn" on:click={openTimeMachine} title="Time Machine">&#128337; Time Machine</button>
    {#if isDirty}
        <span class="status dirty">Unsaved changes...</span>
    {:else}
        <span class="status saved">Saved</span>
    {/if}
</div>

{#if showTimeMachine}
<div class="time-machine-panel">
    <div class="time-machine-header">
        <span>Time Machine</span>
        <button class="close-btn" on:click={() => showTimeMachine = false}>&#x2715;</button>
    </div>
    {#if versionHistory.length === 0}
        <p class="no-history">履歴がありません</p>
    {:else}
        <ul class="version-list">
            {#each versionHistory as entry}
                <li>
                    <span class="version-label">{entry.label}</span>
                    <span class="version-source">{entry.source}</span>
                    <button class="restore-btn" on:click={() => restoreVersion(entry)}>復元</button>
                </li>
            {/each}
        </ul>
    {/if}
</div>
{/if}

<div class="editor-container">
    {#each nodeOrder as id (id)}
        <div data-block-id={id} use:registerBlock style="min-height: 1.5em;">
            {#if visibleBlocks.has(id)}
                {#if blocks[id].blockType === 'mermaid'}
                    <MermaidBlock id={id} plainText={blocks[id].plainText} onSync={handleGraphicalSync} />
                {:else if blocks[id].blockType === 'mathBlock'}
                    <MathBlock id={id} plainText={blocks[id].plainText} onSync={handleGraphicalSync} />
                {:else if blocks[id].blockType === 'typst'}
                    <TypstBlock id={id} plainText={blocks[id].plainText} onSync={handleGraphicalSync} />
                {:else}
                    <Block
                        id={id}
                        blockType={blocks[id].blockType}
                        astContent={blocks[id].astContent}
                        onInput={handleInput}
                        onKeyDown={handleKeyDown}
                        onMove={handleMoveBlock}
                        onFormat={handleFormat}
                        onHistory={handleHistory}
                    />
                {/if}
            {:else}
                <div class="placeholder">...</div>
            {/if}
        </div>
    {/each}
    {#if nodeOrder.length === 0}
        <div class="empty-state">
            <p>No file opened or file is empty.</p>
        </div>
    {/if}
</div>
  
<style>
    .toolbar {
        max-width: 800px;
        margin: 0 auto 1rem auto;
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        background: #2a2a2a;
        border-radius: 8px;
    }
    button {
        padding: 0.5rem 1rem;
        background: #4a90e2;
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
    }
    button:hover {
        background: #357abd;
    }
    .status {
        font-size: 0.9rem;
    }
    .status.dirty { color: #e2a04a; }
    .status.saved { color: #4ae280; }

    .editor-container {
        max-width: 800px;
        margin: 0 auto 2rem auto;
        padding: 2rem;
        background: #1e1e1e;
        color: #e0e0e0;
        border-radius: 8px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.3);
        font-family: 'Inter', system-ui, -apple-system, sans-serif;
        font-size: 1.1rem;
        line-height: 1.6;
    }
    .placeholder {
        color: #444;
        font-style: italic;
    }
    .empty-state {
        text-align: center;
        color: #888;
        padding: 3rem 0;
    }
    .history-btn {
        background: #333;
        border: 1px solid #555;
        color: #ccc;
    }
    .history-btn:hover { background: #444; color: white; }
    .time-machine-panel {
        position: fixed;
        top: 60px;
        right: 1rem;
        width: 320px;
        max-height: 60vh;
        overflow-y: auto;
        background: #252525;
        border: 1px solid #444;
        border-radius: 8px;
        z-index: 100;
        box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    }
    .time-machine-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 1rem;
        border-bottom: 1px solid #333;
        font-weight: 600;
        color: #ddd;
    }
    .close-btn {
        background: transparent;
        border: none;
        color: #888;
        cursor: pointer;
        font-size: 1rem;
        padding: 0;
    }
    .close-btn:hover { color: white; background: transparent; }
    .no-history { color: #666; text-align: center; padding: 1rem; font-size: 0.9rem; }
    .version-list {
        list-style: none;
        margin: 0;
        padding: 0;
    }
    .version-list li {
        display: flex;
        align-items: center;
        padding: 0.5rem 1rem;
        border-bottom: 1px solid #2a2a2a;
        gap: 0.5rem;
    }
    .version-list li:hover { background: #2e2e2e; }
    .version-label { flex: 1; font-size: 0.82rem; color: #ccc; }
    .version-source {
        font-size: 0.7rem;
        padding: 0.1rem 0.35rem;
        border-radius: 3px;
        background: #333;
        color: #888;
    }
    .restore-btn {
        padding: 0.2rem 0.5rem;
        font-size: 0.78rem;
        background: #1a4a7a;
        border: none;
        color: #8bc4ff;
        border-radius: 4px;
        cursor: pointer;
    }
    .restore-btn:hover { background: #1e5a9a; color: white; }
</style>
