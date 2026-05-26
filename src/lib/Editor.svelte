<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount, tick } from 'svelte';
    import { get } from 'svelte/store';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { extractDecorationsFromDOM, getCaretOffset, getSelectionRange, setCaretOffset } from './domUtils';
    import {
        blocks, nodeOrder, isDirty,
        activeBlockId, activeBlockElement,
        nextSeq, blockTypeTag,
        type BlockData, type BlockType, type VersionEntry,
    } from './editorStore';
    import Block from './Block.svelte';
    import MathBlock from './MathBlock.svelte';
    import MermaidBlock from './MermaidBlock.svelte';
    import TypstBlock from './TypstBlock.svelte';

    // ── Menu bar state ────────────────────────────────────────────────────────
    let openMenu: string | null = null;

    function toggleMenu(name: string) {
        openMenu = openMenu === name ? null : name;
    }

    function closeMenu() {
        openMenu = null;
    }

    // ── Pane visibility ───────────────────────────────────────────────────────
    let showDebugPane = false;
    let showRawPane = false;

    // ── Time Machine panel state ──────────────────────────────────────────────
    let showTimeMachine = false;
    let versionHistory: VersionEntry[] = [];

    // ── Virtual rendering (IntersectionObserver) ──────────────────────────────
    let visibleBlocks = new Set<string>();
    let observer: IntersectionObserver;
    let autoSaveTimeout: ReturnType<typeof setTimeout>;

    onMount(() => {
        observer = new IntersectionObserver((entries) => {
            let changed = false;
            entries.forEach(entry => {
                const id = entry.target.getAttribute('data-block-id');
                if (!id) return;
                if (entry.isIntersecting) {
                    if (!visibleBlocks.has(id)) { visibleBlocks.add(id); changed = true; }
                } else {
                    if (visibleBlocks.has(id)) { visibleBlocks.delete(id); changed = true; }
                }
            });
            if (changed) visibleBlocks = visibleBlocks;
        }, { rootMargin: '200px' });

        // Tauri v2 file drag-drop
        let unlistenDrop: (() => void) | undefined;
        getCurrentWindow().onDragDropEvent(async (event) => {
            if (event.payload.type === 'drop') {
                const paths: string[] = (event.payload as any).paths ?? [];
                const mdPath = paths.find(p => p.toLowerCase().endsWith('.md'));
                if (mdPath) {
                    await handleFileDrop(mdPath);
                }
            }
        }).then(fn => { unlistenDrop = fn; }).catch(() => {});

        return () => {
            observer.disconnect();
            unlistenDrop?.();
        };
    });

    function registerBlock(el: HTMLElement) {
        if (observer && el) {
            observer.observe(el);
            visibleBlocks.add(el.getAttribute('data-block-id') as string);
            visibleBlocks = visibleBlocks;
        }
    }

    // ── Toolbar active-format state (selectionchange, 16 ms debounce) ─────────
    let activeFormats = { bold: false, italic: false, code: false };
    let fmtDebounce: ReturnType<typeof setTimeout>;

    function onSelectionChange() {
        clearTimeout(fmtDebounce);
        fmtDebounce = setTimeout(() => {
            const sel = window.getSelection();
            if (!sel || sel.rangeCount === 0) {
                activeFormats = { bold: false, italic: false, code: false };
                return;
            }
            let node: Node | null = sel.getRangeAt(0).startContainer;
            let bold = false, italic = false, code = false;
            while (node) {
                if (node.nodeType === Node.ELEMENT_NODE) {
                    const tag = (node as HTMLElement).tagName?.toLowerCase();
                    if (tag === 'strong' || tag === 'b') bold = true;
                    if (tag === 'em' || tag === 'i') italic = true;
                    if (tag === 'code') code = true;
                }
                node = node.parentNode;
            }
            activeFormats = { bold, italic, code };
        }, 16);
    }

    // ── File operations ───────────────────────────────────────────────────────

    async function openFile() {
        closeMenu();
        try {
            const resp: any = await invoke('open_file');
            if (resp) {
                applyDocumentResponse(resp);
            }
        } catch (err) {
            console.error('Failed to open file:', err);
        }
    }

    async function handleFileDrop(filePath: string) {
        if (get(isDirty)) {
            const ok = confirm('未保存の変更があります。このファイルを開くと失われます。続けますか？');
            if (!ok) return;
        }
        try {
            const resp: any = await invoke('open_file_from_path', { path: filePath });
            if (resp) {
                applyDocumentResponse(resp);
            }
        } catch (err) {
            console.error('Failed to open dropped file:', err);
        }
    }

    function applyDocumentResponse(resp: any) {
        const newBlocks: Record<string, BlockData> = {};
        for (const b of resp.blocks) {
            newBlocks[b.id] = {
                id: b.id,
                blockType: b.blockType,
                astContent: b.astContent,
                plainText: b.plainText || '',
            };
        }
        blocks.set(newBlocks);
        nodeOrder.set(resp.nodeOrder);
        isDirty.set(false);
    }

    async function saveFile() {
        closeMenu();
        try {
            await invoke('save_file');
            isDirty.set(false);
        } catch (err) {
            console.error('Failed to save file:', err);
        }
    }

    function triggerAutoSave() {
        if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
        isDirty.set(true);
        autoSaveTimeout = setTimeout(async () => {
            if (get(isDirty)) {
                try {
                    await invoke('auto_save_silent');
                } catch (err) {
                    console.error('Failed to auto-save:', err);
                }
            }
        }, 3000);
    }

    // ── Block sync ────────────────────────────────────────────────────────────

    async function handleInput(id: string, element: HTMLElement) {
        const currentBlocks = get(blocks);
        if (!currentBlocks[id]) return;

        const plainText = element.textContent || '';
        const decorations = extractDecorationsFromDOM(element);
        const caretOffset = getCaretOffset(element);
        const seq = nextSeq();

        blocks.update(bks => ({ ...bks, [id]: { ...bks[id], astContent: decorations } }));
        triggerAutoSave();

        try {
            await invoke('sync_block', {
                req: { seq, nodeId: id, plainText, decorations, caretOffset }
            });
        } catch (err) {
            console.error('Failed to sync block:', err);
        }
    }

    // ── Enter key: split block ────────────────────────────────────────────────

    function generateBlockId(): string {
        return `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
    }

    async function handleSplitBlock(id: string, element: HTMLElement) {
        // Flush any pending content changes first
        await handleInput(id, element);
        const caretOffset = getCaretOffset(element);
        const newBlockId = generateBlockId();

        try {
            const resp: any = await invoke('split_block', {
                req: { seq: nextSeq(), nodeId: id, caretOffset, newBlockId }
            });
            if (resp) {
                blocks.update(bks => {
                    const updated = { ...bks };
                    updated[id] = {
                        ...updated[id],
                        astContent: resp.originalAstContent,
                        blockType: resp.originalBlockType,
                    };
                    updated[resp.newNodeId] = {
                        id: resp.newNodeId,
                        blockType: resp.newBlockType,
                        astContent: resp.newAstContent,
                        plainText: '',
                    };
                    return updated;
                });
                nodeOrder.set(resp.newNodeOrder);
                triggerAutoSave();

                await tick();
                const newEl = document.querySelector(`[data-block-id="${resp.newNodeId}"]`) as HTMLElement;
                if (newEl) {
                    newEl.focus();
                    setCaretOffset(newEl, 0);
                }
            }
        } catch (err) {
            console.error('split_block failed:', err);
        }
    }

    async function handleKeyDown(id: string, e: KeyboardEvent, element: HTMLElement) {
        if (e.key === 'Enter' && !e.shiftKey) {
            await handleSplitBlock(id, element);
        }
    }

    // ── Block move ────────────────────────────────────────────────────────────

    async function handleMoveBlock(nodeId: string, targetPreviousSiblingId: string | null) {
        nodeOrder.update(order => {
            const idx = order.indexOf(nodeId);
            if (idx === -1) return order;
            const next = [...order];
            next.splice(idx, 1);
            if (targetPreviousSiblingId) {
                const tIdx = next.indexOf(targetPreviousSiblingId);
                next.splice(tIdx + 1, 0, nodeId);
            } else {
                next.unshift(nodeId);
            }
            return next;
        });
        triggerAutoSave();

        try {
            const resp: any = await invoke('move_block', {
                req: { seq: nextSeq(), nodeId, targetParentId: null, targetPreviousSiblingId }
            });
            if (resp && resp.success) {
                nodeOrder.set(resp.newNodeOrder);
            }
        } catch (err) {
            console.error('Failed to move block:', err);
        }
    }

    // ── Graphical block sync ──────────────────────────────────────────────────

    async function handleGraphicalSync(id: string, plainText: string) {
        blocks.update(bks => ({ ...bks, [id]: { ...bks[id], plainText } }));
        triggerAutoSave();
        try {
            await invoke('sync_block', {
                req: { seq: nextSeq(), nodeId: id, plainText, decorations: [], caretOffset: 0 }
            });
        } catch (err) {
            console.error('sync_block failed for graphical block:', err);
        }
    }

    // ── Formatting ────────────────────────────────────────────────────────────

    async function handleFormat(id: string, actionType: string, element: HTMLElement, metaValue?: string) {
        const { start, end } = getSelectionRange(element);
        triggerAutoSave();
        try {
            const resp: any = await invoke('apply_format', {
                req: {
                    seq: nextSeq(), nodeId: id, actionType,
                    selectionStart: start, selectionEnd: end,
                    metaValue: metaValue ?? null
                }
            });
            if (resp) {
                blocks.update(bks => ({
                    ...bks,
                    [resp.nodeId]: { ...bks[resp.nodeId], astContent: resp.astContent, blockType: resp.blockType }
                }));
                await tick();
                const el = document.querySelector(`[data-block-id="${resp.nodeId}"]`) as HTMLElement;
                if (el) setCaretOffset(el, resp.caret.offset);
            }
        } catch (err) {
            console.error('apply_format failed:', err);
        }
    }

    async function formatActiveBlock(actionType: string, metaValue?: string) {
        const id = get(activeBlockId);
        const el = get(activeBlockElement);
        if (!id || !el) return;
        await handleFormat(id, actionType, el, metaValue);
    }

    // ── History ───────────────────────────────────────────────────────────────

    async function handleHistory(type: 'undo' | 'redo') {
        closeMenu();
        try {
            const resp: any = await invoke('trigger_history', {
                req: { seq: nextSeq(), type }
            });
            if (resp) {
                nodeOrder.set(resp.nodeOrder);
                const newBlocks: Record<string, BlockData> = {};
                for (const rb of resp.restoredBlocks) {
                    newBlocks[rb.id] = {
                        id: rb.id,
                        blockType: rb.blockType,
                        astContent: rb.astContent,
                        plainText: rb.markdown || '',
                    };
                }
                blocks.set(newBlocks);
                isDirty.set(true);

                await tick();
                const el = document.querySelector(`[data-block-id="${resp.caret.targetNodeId}"]`) as HTMLElement;
                if (el) setCaretOffset(el, resp.caret.offset);
            }
        } catch (err) {
            console.error('trigger_history failed:', err);
        }
    }

    // ── Time Machine ──────────────────────────────────────────────────────────

    async function applyRestoredBlocks(resp: any) {
        const newBlocks: Record<string, BlockData> = {};
        for (const rb of resp.restoredBlocks) {
            newBlocks[rb.id] = {
                id: rb.id,
                blockType: rb.blockType,
                astContent: rb.astContent,
                plainText: rb.markdown || '',
            };
        }
        blocks.set(newBlocks);
        nodeOrder.set(resp.nodeOrder);
        isDirty.set(true);

        await tick();
        const el = document.querySelector(`[data-block-id="${resp.caret.targetNodeId}"]`) as HTMLElement;
        if (el) setCaretOffset(el, resp.caret.offset);
    }

    async function openTimeMachine() {
        closeMenu();
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
        if (get(isDirty)) {
            const ok = confirm('未保存の変更があります。このバージョンに復元すると失われます。続けますか？');
            if (!ok) return;
        }
        try {
            const resp: any = await invoke('restore_version', {
                req: { seq: nextSeq(), versionId: entry.versionId, source: entry.source }
            });
            if (resp) {
                await applyRestoredBlocks(resp);
                showTimeMachine = false;
            }
        } catch (err) {
            console.error('restore_version failed:', err);
            alert('復元に失敗しました: ' + String(err));
        }
    }

    // ── Toolbar helpers ───────────────────────────────────────────────────────

    function isGraphicalBlock(bt: BlockType | null | undefined): boolean {
        const t = blockTypeTag(bt);
        return t === 'mermaid' || t === 'mathBlock' || t === 'typst';
    }

    $: activeBlock = $activeBlockId ? $blocks[$activeBlockId] : null;
    $: toolbarDisabled = !activeBlock || isGraphicalBlock(activeBlock?.blockType);

    // ── Raw / Debug pane content ──────────────────────────────────────────────

    function astToMarkdown(nodes: any[]): string {
        if (!nodes) return '';
        return nodes.map((n: any) => {
            if (n.type === 'text') return n.text ?? '';
            if (n.type === 'bold') return `**${astToMarkdown(n.children)}**`;
            if (n.type === 'italic') return `*${astToMarkdown(n.children)}*`;
            if (n.type === 'code') return `\`${astToMarkdown(n.children)}\``;
            if (n.type === 'link') return `[${astToMarkdown(n.children)}](${n.href})`;
            return '';
        }).join('');
    }

    function getBlockPrefix(bt: BlockType | null | undefined): string {
        if (!bt) return '';
        const t = bt.type;
        if (t === 'blockQuote') return '> ';
        if (t === 'heading') return '#'.repeat((bt as any).level) + ' ';
        if (t === 'list') return (bt as any).listType === 'ordered' ? '1. ' : '- ';
        if (t === 'codeBlock') return '```' + ((bt as any).language || '') + '\n';
        return '';
    }

    $: debugJson = $activeBlockId && $blocks[$activeBlockId]
        ? JSON.stringify($blocks[$activeBlockId].astContent, null, 2)
        : '(no block selected)';

    $: rawMarkdown = $nodeOrder.map(id => {
        const b = $blocks[id];
        if (!b) return '';
        const t = blockTypeTag(b.blockType);
        if (t === 'mermaid' || t === 'mathBlock' || t === 'typst') return b.plainText || '';
        return getBlockPrefix(b.blockType) + astToMarkdown(b.astContent);
    }).join('\n\n');
</script>

<!-- selectionchange listener for toolbar active-state sync -->
<svelte:document on:selectionchange={onSelectionChange} />
<!-- Click-outside handler to close menus -->
<svelte:window on:click={closeMenu} />

<div class="app-shell">
    <!-- ── Header (MenuBar + Toolbar + Time Machine) ─────────────────────── -->
    <header class="app-header">
        <!-- ── Menu Bar ───────────────────────────────────────────────────── -->
        <div class="menu-bar" on:click|stopPropagation>
            <div class="menu-group" class:active={openMenu === 'file'}>
                <button class="menu-label" on:click={() => toggleMenu('file')}>File</button>
                {#if openMenu === 'file'}
                <div class="menu-dropdown">
                    <button on:click={openFile}>Open...</button>
                    <button on:click={saveFile}>Save</button>
                </div>
                {/if}
            </div>

            <div class="menu-group" class:active={openMenu === 'edit'}>
                <button class="menu-label" on:click={() => toggleMenu('edit')}>Edit</button>
                {#if openMenu === 'edit'}
                <div class="menu-dropdown">
                    <button on:click={() => handleHistory('undo')}>Undo</button>
                    <button on:click={() => handleHistory('redo')}>Redo</button>
                </div>
                {/if}
            </div>

            <div class="menu-group" class:active={openMenu === 'view'}>
                <button class="menu-label" on:click={() => toggleMenu('view')}>View</button>
                {#if openMenu === 'view'}
                <div class="menu-dropdown">
                    <button on:click={openTimeMachine}>&#128337; Time Machine</button>
                    <hr class="menu-divider" />
                    <button on:click={() => { showDebugPane = !showDebugPane; closeMenu(); }}>
                        {showDebugPane ? '✓' : '　'} Debug (AST)
                    </button>
                    <button on:click={() => { showRawPane = !showRawPane; closeMenu(); }}>
                        {showRawPane ? '✓' : '　'} Raw Markdown
                    </button>
                </div>
                {/if}
            </div>

            <div class="menu-group">
                <button class="menu-label" on:click={() => toggleMenu('settings')}>Settings</button>
            </div>

            <div class="menu-status">
                {#if $isDirty}
                    <span class="status dirty" title="Unsaved changes">●</span>
                {:else}
                    <span class="status saved" title="Saved">●</span>
                {/if}
            </div>
        </div>

        <!-- ── Format Toolbar ─────────────────────────────────────────────── -->
        <div class="format-toolbar">
            <button
                class="fmt-btn"
                class:active={activeFormats.bold}
                disabled={toolbarDisabled}
                title="Bold (Ctrl+B)"
                on:mousedown|preventDefault={() => formatActiveBlock('bold')}>
                <b>B</b>
            </button>
            <button
                class="fmt-btn"
                class:active={activeFormats.italic}
                disabled={toolbarDisabled}
                title="Italic (Ctrl+I)"
                on:mousedown|preventDefault={() => formatActiveBlock('italic')}>
                <i>I</i>
            </button>
            <button
                class="fmt-btn"
                class:active={activeFormats.code}
                disabled={toolbarDisabled}
                title="Inline Code (Ctrl+E)"
                on:mousedown|preventDefault={() => formatActiveBlock('code')}>
                <code>&#96;&#96;</code>
            </button>

            <span class="tb-divider"></span>

            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 1"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '1')}>H1</button>
            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 2"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '2')}>H2</button>
            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 3"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '3')}>H3</button>
            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 4"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '4')}>H4</button>
            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 5"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '5')}>H5</button>
            <button class="fmt-btn" disabled={toolbarDisabled} title="Heading 6"
                on:mousedown|preventDefault={() => formatActiveBlock('heading', '6')}>H6</button>

            <span class="tb-divider"></span>

            <button class="fmt-btn" disabled={toolbarDisabled} title="Unordered List"
                on:mousedown|preventDefault={() => formatActiveBlock('list')}>
                &#8801;
            </button>
        </div>

        <!-- ── Time Machine Panel ─────────────────────────────────────────── -->
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
    </header>

    <!-- ── Editor Main (3-pane flex row) ─────────────────────────────────── -->
    <main class="editor-main">
        <!-- Left pane: Debug / AST view -->
        {#if showDebugPane}
        <aside class="pane pane-debug">
            <div class="pane-title">AST Debug</div>
            <pre class="pane-content">{debugJson}</pre>
        </aside>
        {/if}

        <!-- Center pane: editor -->
        <div class="pane pane-center">
            <div class="editor-container">
                {#each $nodeOrder as id (id)}
                    <div data-block-id={id} use:registerBlock style="min-height: 1.5em;">
                        {#if visibleBlocks.has(id)}
                            {#if blockTypeTag($blocks[id]?.blockType) === 'mermaid'}
                                <MermaidBlock id={id} plainText={$blocks[id].plainText} onSync={handleGraphicalSync} />
                            {:else if blockTypeTag($blocks[id]?.blockType) === 'mathBlock'}
                                <MathBlock id={id} plainText={$blocks[id].plainText} onSync={handleGraphicalSync} />
                            {:else if blockTypeTag($blocks[id]?.blockType) === 'typst'}
                                <TypstBlock id={id} plainText={$blocks[id].plainText} onSync={handleGraphicalSync} />
                            {:else}
                                <Block
                                    id={id}
                                    blockType={$blocks[id].blockType}
                                    astContent={$blocks[id].astContent}
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
                {#if $nodeOrder.length === 0}
                    <div class="empty-state">
                        <p>No file opened. Use File → Open or drop a .md file here.</p>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Right pane: Raw Markdown view -->
        {#if showRawPane}
        <aside class="pane pane-raw">
            <div class="pane-title">Raw Markdown</div>
            <pre class="pane-content">{rawMarkdown}</pre>
        </aside>
        {/if}
    </main>
</div>

<style>
    /* ── App Shell ───────────────────────────────────────────────────────── */
    .app-shell {
        display: flex;
        flex-direction: column;
        height: 100%;
        width: 100%;
    }

    /* ── App Header ──────────────────────────────────────────────────────── */
    .app-header {
        position: relative;
        width: 100%;
        display: flex;
        flex-direction: column;
        background: #1a1a1a;
        border-bottom: 1px solid #333;
        z-index: 200;
        flex-shrink: 0;
    }

    /* ── Editor Main — 3-pane row ────────────────────────────────────────── */
    .editor-main {
        flex: 1;
        min-height: 0;
        overflow: hidden;       /* panes scroll independently */
        display: flex;
        flex-direction: row;
    }

    /* ── Shared pane base ────────────────────────────────────────────────── */
    .pane {
        overflow-y: auto;
        display: flex;
        flex-direction: column;
    }

    /* ── Side panes (debug / raw) ────────────────────────────────────────── */
    .pane-debug {
        width: 30%;
        flex-shrink: 0;
        background: #111;
        border-right: 1px solid #2a2a2a;
    }
    .pane-raw {
        width: 30%;
        flex-shrink: 0;
        background: #111;
        border-left: 1px solid #2a2a2a;
    }
    .pane-title {
        font-size: 0.75rem;
        font-weight: 600;
        color: #666;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        padding: 0.5rem 0.75rem 0.25rem;
        border-bottom: 1px solid #222;
        flex-shrink: 0;
    }
    .pane-content {
        flex: 1;
        margin: 0;
        padding: 0.75rem;
        font-family: 'Consolas', 'SF Mono', monospace;
        font-size: 0.72rem;
        line-height: 1.5;
        color: #8c8;
        white-space: pre-wrap;
        word-break: break-all;
        overflow-wrap: anywhere;
    }
    .pane-raw .pane-content { color: #8bc; }

    /* ── Center pane ─────────────────────────────────────────────────────── */
    .pane-center {
        flex: 1;
        min-width: 0;
        overflow-y: auto;
        display: flex;
        justify-content: center;
        align-items: flex-start;
    }

    /* ── Menu Bar ────────────────────────────────────────────────────────── */
    .menu-bar {
        width: 100%;
        padding: 0 0.5rem;
        display: flex;
        align-items: center;
        gap: 0;
        background: #1a1a1a;
        border-bottom: 1px solid #333;
        font-size: 0.88rem;
        position: relative;
        z-index: 200;
    }
    .menu-group {
        position: relative;
    }
    .menu-label {
        background: transparent;
        border: none;
        color: #bbb;
        padding: 0.45rem 0.75rem;
        cursor: pointer;
        border-radius: 4px 4px 0 0;
        font-size: 0.88rem;
    }
    .menu-label:hover, .menu-group.active .menu-label {
        background: #2e2e2e;
        color: #fff;
    }
    .menu-dropdown {
        position: absolute;
        top: 100%;
        left: 0;
        min-width: 160px;
        background: #2a2a2a;
        border: 1px solid #444;
        border-radius: 0 4px 4px 4px;
        box-shadow: 0 6px 20px rgba(0,0,0,0.5);
        display: flex;
        flex-direction: column;
        z-index: 300;
    }
    .menu-dropdown button {
        background: transparent;
        border: none;
        color: #ccc;
        padding: 0.45rem 1rem;
        text-align: left;
        cursor: pointer;
        font-size: 0.88rem;
        border-radius: 0;
        width: 100%;
    }
    .menu-dropdown button:hover {
        background: #3a3a3a;
        color: #fff;
    }
    .menu-divider {
        border: none;
        border-top: 1px solid #333;
        margin: 0.2rem 0;
    }
    .menu-status {
        margin-left: auto;
        padding: 0 0.75rem;
    }
    .status { font-size: 0.7rem; }
    .status.dirty { color: #e2a04a; }
    .status.saved  { color: #4ae280; }

    /* ── Format Toolbar ──────────────────────────────────────────────────── */
    .format-toolbar {
        width: 100%;
        padding: 0.3rem 0.75rem;
        display: flex;
        align-items: center;
        gap: 0.2rem;
        background: #222;
        border-bottom: 1px solid #333;
        flex-wrap: wrap;
    }
    .fmt-btn {
        background: transparent;
        border: 1px solid transparent;
        color: #bbb;
        padding: 0.2rem 0.5rem;
        cursor: pointer;
        border-radius: 4px;
        font-size: 0.9rem;
        min-width: 28px;
        text-align: center;
        line-height: 1.4;
        transition: background 0.1s, border-color 0.1s, color 0.1s;
    }
    .fmt-btn:hover:not(:disabled) {
        background: #333;
        border-color: #555;
        color: #fff;
    }
    .fmt-btn.active {
        background: #1a4a7a;
        border-color: #4a90e2;
        color: #8bc4ff;
    }
    .fmt-btn:disabled {
        color: #444;
        cursor: default;
    }
    .tb-divider {
        width: 1px;
        height: 16px;
        background: #444;
        margin: 0 0.25rem;
    }

    /* ── Editor Container ────────────────────────────────────────────────── */
    .editor-container {
        max-width: min(800px, 100%);
        width: 100%;
        margin: 2rem auto;
        padding: 2rem;
        background: #1e1e1e;
        color: #e0e0e0;
        border-radius: 8px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.3);
        font-family: 'Inter', system-ui, -apple-system, sans-serif;
        font-size: 1.1rem;
        line-height: 1.6;
        box-sizing: border-box;
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

    /* ── Time Machine Panel ──────────────────────────────────────────────── */
    .time-machine-panel {
        position: absolute;
        top: 100%;
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
    .version-list { list-style: none; margin: 0; padding: 0; }
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
