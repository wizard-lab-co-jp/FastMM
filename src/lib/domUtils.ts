export type InlineDecorationNode =
    | { type: 'text'; key: string; text: string }
    | { type: 'bold'; key: string; children: InlineDecorationNode[] }
    | { type: 'italic'; key: string; children: InlineDecorationNode[] }
    | { type: 'code'; key: string; children: InlineDecorationNode[] }
    | { type: 'link'; key: string; href: string; children: InlineDecorationNode[] };

// Returns the text length of a node, recursively. <br> counts as 1.
function getNodeTextLength(node: Node): number {
    if (!node) return 0;
    if (node.nodeType === Node.TEXT_NODE) return node.textContent?.length || 0;
    if (node.nodeType === Node.ELEMENT_NODE && (node as Element).nodeName === 'BR') return 1;
    let len = 0;
    for (let i = 0; i < node.childNodes.length; i++) {
        len += getNodeTextLength(node.childNodes[i]);
    }
    return len;
}

function getChildIndex(parent: Node, child: Node): number {
    let i = 0;
    let cur = parent.firstChild;
    while (cur) {
        if (cur === child) return i;
        i++;
        cur = cur.nextSibling;
    }
    return -1;
}

/**
 * Counts characters from start of container up to (endNode, endOffset).
 * For TEXT_NODE: endOffset is a character offset.
 * For ELEMENT_NODE: endOffset is a child index.
 * <br> is counted as 1 character.
 */
function countCharsToPosition(container: HTMLElement, endNode: Node, endOffset: number): number {
    let count = 0;

    function walk(node: Node): boolean {
        if (node === endNode) {
            if (node.nodeType === Node.TEXT_NODE) {
                count += endOffset;
            } else {
                // ELEMENT_NODE: endOffset is child index — sum lengths of preceding children
                for (let i = 0; i < endOffset; i++) {
                    count += getNodeTextLength(node.childNodes[i]);
                }
            }
            return true;
        }

        if (node.nodeType === Node.TEXT_NODE) {
            count += node.textContent?.length || 0;
            return false;
        }

        if (node.nodeType === Node.ELEMENT_NODE) {
            if ((node as Element).nodeName === 'BR') {
                count += 1;
                return false;
            }
            for (let i = 0; i < node.childNodes.length; i++) {
                if (walk(node.childNodes[i])) return true;
            }
        }

        return false;
    }

    for (let i = 0; i < container.childNodes.length; i++) {
        if (walk(container.childNodes[i])) break;
    }

    return count;
}

/**
 * Extracts the InlineDecorationNode tree from a DOM element.
 * Uses position-based stable keys (prefix_index) to prevent Svelte from
 * recreating DOM nodes on every keystroke, which would break IME and selection.
 */
export function extractDecorationsFromDOM(node: Node, prefix: string = 'r'): InlineDecorationNode[] {
    const result: InlineDecorationNode[] = [];

    for (let i = 0; i < node.childNodes.length; i++) {
        const child = node.childNodes[i];
        const childKey = `${prefix}_${i}`;

        if (child.nodeType === Node.TEXT_NODE) {
            const text = child.textContent || '';
            if (text.length > 0) {
                result.push({ type: 'text', key: childKey, text });
            }
        } else if (child.nodeType === Node.ELEMENT_NODE) {
            const el = child as HTMLElement;
            const key = el.dataset.key || childKey;
            const tag = el.tagName.toLowerCase();

            if (tag === 'br') {
                result.push({ type: 'text', key, text: '\n' });
                continue;
            }

            if (tag === 'strong' || tag === 'b') {
                result.push({ type: 'bold', key, children: extractDecorationsFromDOM(el, childKey) });
            } else if (tag === 'em' || tag === 'i') {
                result.push({ type: 'italic', key, children: extractDecorationsFromDOM(el, childKey) });
            } else if (tag === 'code') {
                result.push({ type: 'code', key, children: extractDecorationsFromDOM(el, childKey) });
            } else if (tag === 'a') {
                result.push({ type: 'link', key, href: el.getAttribute('href') || '', children: extractDecorationsFromDOM(el, childKey) });
            } else if (tag === 'span') {
                result.push(...extractDecorationsFromDOM(el, childKey));
            } else {
                result.push({ type: 'text', key, text: el.textContent || '' });
            }
        }
    }

    return result;
}

/**
 * Gets the current caret offset (UTF-16 code units) relative to the container.
 * Correctly handles both TEXT_NODE and ELEMENT_NODE selection endpoints,
 * and counts <br> as 1 character.
 */
export function getCaretOffset(container: HTMLElement): number {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return 0;

    const range = selection.getRangeAt(0);
    return countCharsToPosition(container, range.endContainer as Node, range.endOffset);
}

/**
 * Restores the caret position within the container at the given UTF-16 offset.
 * Visits only text nodes and <br> elements (each counted as 1 char) to avoid
 * double-counting text content from parent elements.
 * Falls back to container start when the block is empty.
 */
export function setCaretOffset(container: HTMLElement, offset: number) {
    const sel = window.getSelection();
    if (!sel) return;

    let remaining = offset;

    function findPosition(node: Node): { node: Node; offset: number } | null {
        if (node.nodeType === Node.TEXT_NODE) {
            const len = node.textContent?.length || 0;
            if (remaining <= len) {
                return { node, offset: remaining };
            }
            remaining -= len;
            return null;
        }

        if (node.nodeType === Node.ELEMENT_NODE) {
            if ((node as Element).nodeName === 'BR') {
                if (remaining === 0) {
                    const parent = node.parentNode!;
                    return { node: parent, offset: getChildIndex(parent, node) };
                }
                remaining -= 1;
                if (remaining === 0) {
                    const parent = node.parentNode!;
                    return { node: parent, offset: getChildIndex(parent, node) + 1 };
                }
                return null;
            }
            for (let i = 0; i < node.childNodes.length; i++) {
                const result = findPosition(node.childNodes[i]);
                if (result) return result;
            }
        }

        return null;
    }

    let position: { node: Node; offset: number } | null = null;
    for (let i = 0; i < container.childNodes.length; i++) {
        position = findPosition(container.childNodes[i]);
        if (position) break;
    }

    const range = document.createRange();

    if (position) {
        range.setStart(position.node, position.offset);
    } else {
        // Fallback: end of last text node, or container start if empty
        const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
        let lastNode: Node | null = null;
        let cur = walker.nextNode();
        while (cur) { lastNode = cur; cur = walker.nextNode(); }

        if (lastNode) {
            range.setStart(lastNode, lastNode.textContent?.length || 0);
        } else {
            range.setStart(container, 0);
        }
    }

    range.collapse(true);
    sel.removeAllRanges();
    sel.addRange(range);
}

export interface SelectionRange {
    start: number;
    end: number;
    isCollapsed: boolean;
}

export function getSelectionRange(container: HTMLElement): SelectionRange {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) {
        return { start: 0, end: 0, isCollapsed: true };
    }
    const range = selection.getRangeAt(0);

    const start = countCharsToPosition(container, range.startContainer as Node, range.startOffset);
    const end = countCharsToPosition(container, range.endContainer as Node, range.endOffset);

    return {
        start: Math.min(start, end),
        end: Math.max(start, end),
        isCollapsed: range.collapsed
    };
}
