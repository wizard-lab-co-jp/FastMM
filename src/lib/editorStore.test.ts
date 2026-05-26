import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { blocks, nodeOrder, isDirty, activeBlockId, nextSeq, resetSeq } from './editorStore';

describe('editorStore tests', () => {
    beforeEach(() => {
        blocks.set({});
        nodeOrder.set([]);
        isDirty.set(false);
        activeBlockId.set(null);
        resetSeq();
    });

    it('should initialize and update blocks store', () => {
        expect(get(blocks)).toEqual({});
        
        blocks.set({
            'b1': {
                id: 'b1',
                blockType: 'paragraph',
                astContent: [],
                plainText: 'hello'
            }
        });
        
        expect(get(blocks)['b1'].plainText).toBe('hello');
    });

    it('should track monotonicity of nextSeq', () => {
        expect(nextSeq()).toBe(1);
        expect(nextSeq()).toBe(2);
        expect(nextSeq()).toBe(3);
        
        resetSeq();
        expect(nextSeq()).toBe(1);
    });

    it('should track dirty and nodeOrder states', () => {
        expect(get(isDirty)).toBe(false);
        isDirty.set(true);
        expect(get(isDirty)).toBe(true);

        expect(get(nodeOrder)).toEqual([]);
        nodeOrder.set(['b1', 'b2']);
        expect(get(nodeOrder)).toEqual(['b1', 'b2']);
    });
});
