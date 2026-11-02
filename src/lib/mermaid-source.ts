import type { Element } from 'hast';

export function mermaidSource(node: Element): string | undefined {
  if (node.tagName !== 'pre') return;
  const code = node.children[0];
  if (code?.type !== 'element' || code.tagName !== 'code') return;
  const classes = code.properties.className;
  if (!Array.isArray(classes) || !classes.includes('language-mermaid')) return;
  return code.children
    .map((child) => (child.type === 'text' ? child.value : ''))
    .join('')
    .trimEnd();
}
