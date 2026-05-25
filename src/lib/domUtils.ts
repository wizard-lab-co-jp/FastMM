export type InlineDecorationNode =
    | { type: 'text'; key: string; text: string }
    | { type: 'bold'; key: string; children: InlineDecorationNode[] }
    | { type: 'italic'; key: string; children: InlineDecorationNode[] }
    | { type: 'code'; key: string; children: InlineDecorationNode[] }
    | { type: 'link'; key: string; href: string; children: InlineDecorationNode[] };

function generateKey() {
    return 'k_' + Math.random().toString(36).substring(2, 9);
}

/**
 * Extracts the InlineDecorationNode tree from a DOM element.
 */
export function extractDecorationsFromDOM(node: Node): InlineDecorationNode[] {
    const result: InlineDecorationNode[] = [];

    for (let i = 0; i < node.childNodes.length; i++) {
        const child = node.childNodes[i];

        if (child.nodeType === Node.TEXT_NODE) {
            const text = child.textContent || '';
            if (text.length > 0) {
                result.push({ type: 'text', key: generateKey(), text });
            }
        } else if (child.nodeType === Node.ELEMENT_NODE) {
            const el = child as HTMLElement;
            const key = el.dataset.key || generateKey();
            const tag = el.tagName.toLowerCase();

            // Ignore elements with zero width/invisible unless they have content
            if (tag === 'br') {
                result.push({ type: 'text', key, text: '\n' });
                continue;
            }

            if (tag === 'strong' || tag === 'b') {
                result.push({ type: 'bold', key, children: extractDecorationsFromDOM(el) });
            } else if (tag === 'em' || tag === 'i') {
                result.push({ type: 'italic', key, children: extractDecorationsFromDOM(el) });
            } else if (tag === 'code') {
                result.push({ type: 'code', key, children: extractDecorationsFromDOM(el) });
            } else if (tag === 'a') {
                result.push({ type: 'link', key, href: el.getAttribute('href') || '', children: extractDecorationsFromDOM(el) });
            } else if (tag === 'span') {
                // If the span has a specific decoration, we handle it if needed.
                // Otherwise, treat its children normally (or just text).
                result.push(...extractDecorationsFromDOM(el));
            } else {
                // Fallback for divs, spans without special tags
                result.push({ type: 'text', key, text: el.textContent || '' });
            }
        }
    }

    return result;
}

/**
 * Gets the current caret offset (UTF-16 code units) relative to the container.
 */
export function getCaretOffset(container: HTMLElement): number {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return 0;

    const range = selection.getRangeAt(0);
    let offset = 0;

    const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
    let currentNode = walker.nextNode();

    while (currentNode) {
        if (currentNode === range.endContainer) {
            offset += range.endOffset;
            break;
        } else {
            offset += currentNode.textContent?.length || 0;
        }
        currentNode = walker.nextNode();
    }

    return offset;
}

/**
 * Restores the caret position within the container at the given UTF-16 offset.
 */
export function setCaretOffset(container: HTMLElement, offset: number) {
    const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
    let currentNode = walker.nextNode();
    let currentOffset = 0;

    while (currentNode) {
        const textLen = currentNode.textContent?.length || 0;
        if (currentOffset + textLen >= offset) {
            const range = document.createRange();
            const pos = offset - currentOffset;
            // Ensure pos is within text bounds
            range.setStart(currentNode, Math.min(pos, textLen));
            range.collapse(true);

            const sel = window.getSelection();
            if (sel) {
                sel.removeAllRanges();
                sel.addRange(range);
            }
            return;
        }
        currentOffset += textLen;
        currentNode = walker.nextNode();
    }

    // If offset is beyond all text, place caret at the end of the last text node
    const lastTextNode = walker.previousNode();
    if (lastTextNode) {
        const range = document.createRange();
        range.setStart(lastTextNode, lastTextNode.textContent?.length || 0);
        range.collapse(true);
        const sel = window.getSelection();
        if (sel) {
            sel.removeAllRanges();
            sel.addRange(range);
        }
    }
}

export interface SelectionRange {
    start: number;   // UTF-16 code unit offset
    end: number;     // UTF-16 code unit offset
    isCollapsed: boolean;
}

export function getSelectionRange(container: HTMLElement): SelectionRange {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) {
        return { start: 0, end: 0, isCollapsed: true };
    }
    const range = selection.getRangeAt(0);

    function computeOffsetInContainer(container: Node, targetNode: Node, targetOffset: number): number {
        let offset = 0;
        const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
        let currentNode = walker.nextNode();

        while (currentNode) {
            if (currentNode === targetNode) {
                offset += targetOffset;
                break;
            } else {
                offset += currentNode.textContent?.length || 0;
            }
            currentNode = walker.nextNode();
        }
        return offset;
    }

    const start = computeOffsetInContainer(container, range.startContainer, range.startOffset);
    const end = computeOffsetInContainer(container, range.endContainer, range.endOffset);
    
    return {
        start: Math.min(start, end),
        end: Math.max(start, end),
        isCollapsed: range.collapsed
    };
}
