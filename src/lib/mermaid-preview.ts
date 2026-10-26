let nextDiagramId = 0;
let renderQueue: Promise<void> = Promise.resolve();
let mermaidModule: Promise<typeof import('mermaid')> | undefined;

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
): null | void {
  if (language.toLowerCase() !== 'mermaid' || !content.trim()) return null;

  // Mermaid uses the DOM while rendering. Keep renders in order so an older
  // edit cannot replace a newer preview of the same code block.
  renderQueue = renderQueue.then(async () => {
    try {
      const mermaid = (await loadMermaid()).default;
      const { svg } = await mermaid.render(`vivin-mermaid-${++nextDiagramId}`, content);
      applyPreview(svg);
    } catch (error) {
      const message = document.createElement('div');
      message.className = 'vivin-mermaid-error';
      message.textContent = error instanceof Error ? error.message : String(error);
      applyPreview(message);
    }
  });
}
