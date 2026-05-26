// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { extractDecorationsFromDOM, getCaretOffset, setCaretOffset } from './domUtils';
import type { InlineDecorationNode } from './domUtils';

describe('domUtils', () => {
    let container: HTMLElement;

    beforeEach(() => {
        // Setup JSDOM container
        document.body.innerHTML = '';
        container = document.createElement('div');
        container.contentEditable = 'true';
        document.body.appendChild(container);
    });

    it('extracts basic text', () => {
        container.innerHTML = 'Hello world';
        const decs = extractDecorationsFromDOM(container);
        expect(decs.length).toBe(1);
        expect(decs[0].type).toBe('text');
        if (decs[0].type === 'text') {
            expect(decs[0].text).toBe('Hello world');
            expect(decs[0].key).toBeDefined();
        }
    });

    it('extracts nested decorations (bold, italic)', () => {
        container.innerHTML = 'Hello <strong data-key="k1">bold <em data-key="k2">italic</em></strong>!';
        const decs = extractDecorationsFromDOM(container);
        expect(decs.length).toBe(3); // text, bold, text
        
        expect(decs[0].type).toBe('text');
        if (decs[0].type === 'text') expect(decs[0].text).toBe('Hello ');

        expect(decs[1].type).toBe('bold');
        if (decs[1].type === 'bold') {
            expect(decs[1].key).toBe('k1');
            expect(decs[1].children.length).toBe(2); // "bold " and italic
            
            const firstChild = decs[1].children[0];
            expect(firstChild.type).toBe('text');
            if (firstChild.type === 'text') expect(firstChild.text).toBe('bold ');

            const secondChild = decs[1].children[1];
            expect(secondChild.type).toBe('italic');
            if (secondChild.type === 'italic') {
                expect(secondChild.key).toBe('k2');
                expect(secondChild.children.length).toBe(1);
                const italicText = secondChild.children[0];
                if (italicText.type === 'text') expect(italicText.text).toBe('italic');
            }
        }

        expect(decs[2].type).toBe('text');
        if (decs[2].type === 'text') expect(decs[2].text).toBe('!');
    });

    it('handles br tags as line breaks', () => {
        container.innerHTML = 'Line 1<br>Line 2';
        const decs = extractDecorationsFromDOM(container);
        expect(decs.length).toBe(3);
        if (decs[1].type === 'text') expect(decs[1].text).toBe('\n');
    });

    it('getCaretOffset and setCaretOffset work with surrogate pairs (UTF-16)', () => {
        // Emoji is 2 UTF-16 code units. '日' is 1 UTF-16 code unit.
        container.innerHTML = 'あ👨‍👩‍👧‍👦い'; // The family emoji is complex. Let's use a simpler one.
        container.innerHTML = 'あ😀い'; 
        // 'あ' (1), '😀' (2), 'い' (1) -> total length = 4

        // Set caret to index 3 (between 😀 and い)
        setCaretOffset(container, 3);
        
        // Now get the offset and verify it is 3
        const offset = getCaretOffset(container);
        expect(offset).toBe(3);
    });

    it('getCaretOffset correctly calculates offset across multiple text nodes', () => {
        container.innerHTML = '<span>text1</span><b>text2</b>';
        // Set caret after 'text2' -> offset should be 5 + 5 = 10
        setCaretOffset(container, 10);
        const offset = getCaretOffset(container);
        expect(offset).toBe(10);
    });
});
