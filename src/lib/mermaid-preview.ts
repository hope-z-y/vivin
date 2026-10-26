let nextDiagramId = 0;
let renderQueue: Promise<void> = Promise.resolve();
let mermaidModule: Promise<typeof import('mermaid')> | undefined;
const previewCache = new Map<string, string>();

export function getCachedMermaidPreview(content: string): string | undefined {
  return previewCache.get(content);
}

function loadMermaid() {
  mermaidModule ??= import('mermaid').then((module) => {
    module.default.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
      suppressErrorRendering: true,
      flowchart: { useMaxWidth: false },
    });
    return module;
  });
  return mermaidModule;
}

export function renderMermaidPreview(
  language: string,
  content: string,
  applyPreview: (value: null | string | HTMLElement) => void,
): null | string | void {
  if (language.toLowerCase() !== 'mermaid' || !content.trim()) return null;

  const cached = previewCache.get(content);
  if (cached) return cached;

  // Mermaid uses the DOM while rendering. Keep renders in order so an older
  // edit cannot replace a newer preview of the same code block.
  renderQueue = renderQueue.then(async () => {
    try {
      const mermaid = (await loadMermaid()).default;
      const { svg } = await mermaid.render(`vivin-mermaid-${++nextDiagramId}`, content);
      previewCache.set(content, svg);
      if (previewCache.size > 128) {
        const oldest = previewCache.keys().next().value;
        if (oldest !== undefined) previewCache.delete(oldest);
      }
      applyPreview(svg);
    } catch (error) {
      const message = document.createElement('div');
      message.className = 'vivin-mermaid-error';
      message.textContent = error instanceof Error ? error.message : String(error);
      applyPreview(message);
    }
  });
}
