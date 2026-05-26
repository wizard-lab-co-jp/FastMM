import { writable } from 'svelte/store';
import type { InlineDecorationNode } from './domUtils';

// Mirrors the Rust BlockType enum serialized with serde tag="type" rename_all="camelCase".
export type BlockType =
    | { type: 'paragraph' }
    | { type: 'mathBlock' }
    | { type: 'mermaid' }
    | { type: 'typst' }
    | { type: 'blockQuote' }
    | { type: 'heading'; level: number }
    | { type: 'codeBlock'; language: string }
    | { type: 'list'; listType: string; indentLevel: number; parentListId: string | null };

export function blockTypeTag(bt: BlockType | null | undefined): string {
    if (!bt) return 'paragraph';
    return bt.type;
}

export type BlockData = {
    id: string;
    blockType: BlockType;
    astContent: InlineDecorationNode[];
    plainText: string;
};

export type VersionEntry = { versionId: string; source: string; label: string };

// ── Core editor state stores ──────────────────────────────────────────────────

export const blocks = writable<Record<string, BlockData>>({});
export const nodeOrder = writable<string[]>([]);
export const isDirty = writable(false);

// ── Selection / active block ──────────────────────────────────────────────────

export const activeBlockId = writable<string | null>(null);
export const activeBlockElement = writable<HTMLElement | null>(null);

// ── Monotonic sequence counter (not reactive – no UI depends on its value) ───

let _seq = 0;

/** Returns the next globally unique, monotonically increasing sequence number.
 *  All Rust RPC calls must use this instead of per-block counters or Date.now(). */
export function nextSeq(): number {
    return ++_seq;
}

export function resetSeq(): void {
    _seq = 0;
}
