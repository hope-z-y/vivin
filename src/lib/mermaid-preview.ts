let nextDiagramId = 0;
let renderQueue: Promise<void> = Promise.resolve();
let mermaidModule: Promise<typeof import('mermaid')> | undefined;
const previewCache = new Map<string, string>();

function isDark() {
  return document.querySelector('.theme-dark') !== null;
}

export function getCachedMermaidPreview(content: string): string | undefined {
  return previewCache.get(`${isDark()}:${content}`);
}

export function renderMermaidSvg(content: string, dark = isDark()): Promise<string> {
  const key = `${dark}:${content}`;
  const cached = previewCache.get(key);
  if (cached) return Promise.resolve(cached);
  // Configuration and DOM rendering share one queue, including theme changes.
  const result = renderQueue.then(async () => {
    mermaidModule ??= import('mermaid');
    const mermaid = (await mermaidModule).default;
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
      suppressErrorRendering: true,
      maxTextSize: 500_000,
      theme: 'base',
      fontFamily:
        '-apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif',
      themeVariables: {
        darkMode: dark,
        fontSize: '15px',
        primaryColor: dark ? '#263b54' : '#edf4ff',
        primaryTextColor: dark ? '#e5edf8' : '#27364b',
        primaryBorderColor: dark ? '#6b8bad' : '#8ca8ca',
        secondaryColor: dark ? '#253c3b' : '#eff7f5',
        tertiaryColor: dark ? '#293344' : '#f6f8fb',
        lineColor: dark ? '#94a8c2' : '#73849b',
        textColor: dark ? '#e5edf8' : '#27364b',
        clusterBkg: dark ? '#202c3b' : '#f6f8fb',
        clusterBorder: dark ? '#485b73' : '#d7e0ec',
        edgeLabelBackground: dark ? '#1b2635' : '#ffffff',
      },
      flowchart: {
        useMaxWidth: true,
        curve: 'linear',
        padding: 16,
        nodeSpacing: 28,
        rankSpacing: 36,
        wrappingWidth: 160,
      },
      themeCSS:
        '.cluster rect, .node rect, .node polygon, .node circle, .node path { filter: none !important; } .edgeLabel { background-color: transparent; }',
    });
    const { svg } = await mermaid.render(`vivin-mermaid-${++nextDiagramId}`, content);
    previewCache.set(key, svg);
    if (previewCache.size > 128) {
      const oldest = previewCache.keys().next().value;
      if (oldest !== undefined) previewCache.delete(oldest);
    }
    return svg;
  });
  renderQueue = result.then(
    () => {},
    () => {},
  );
  return result;
}

export function renderMermaidPreview(
  language: string,
  content: string,
  applyPreview: (value: null | string | HTMLElement) => void,
): null | string | void {
  if (language.toLowerCase() !== 'mermaid' || !content.trim()) return null;
  const cached = getCachedMermaidPreview(content);
  if (cached) return cached;
  void renderMermaidSvg(content).then(applyPreview, (error: unknown) => {
    const message = document.createElement('div');
    message.className = 'vivin-mermaid-error';
    message.textContent = error instanceof Error ? error.message : String(error);
    applyPreview(message);
  });
}
