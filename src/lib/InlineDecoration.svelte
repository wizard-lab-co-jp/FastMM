<script lang="ts">
    import type { InlineDecorationNode } from './domUtils';
    export let node: InlineDecorationNode;
</script>

{#if node.type === 'text'}
    <span data-key={node.key} data-decoration="text">{node.text}</span>
{:else if node.type === 'bold'}
    <strong data-key={node.key}>
        {#each node.children as child (child.key)}
            <svelte:self node={child} />
        {/each}
    </strong>
{:else if node.type === 'italic'}
    <em data-key={node.key}>
        {#each node.children as child (child.key)}
            <svelte:self node={child} />
        {/each}
    </em>
{:else if node.type === 'code'}
    <code data-key={node.key}>
        {#each node.children as child (child.key)}
            <svelte:self node={child} />
        {/each}
    </code>
{:else if node.type === 'link'}
    <a data-key={node.key} href={node.href}>
        {#each node.children as child (child.key)}
            <svelte:self node={child} />
        {/each}
    </a>
{/if}
