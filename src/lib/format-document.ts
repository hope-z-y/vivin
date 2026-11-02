export function formatDocument(content: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./markdown-format.worker.ts', import.meta.url), {
      type: 'module',
    });
    const timer = setTimeout(() => {
      worker.terminate();
      reject(new Error('Markdown formatting timed out'));
    }, 60_000);
    worker.onmessage = (event: MessageEvent<{ content: string; error?: string }>) => {
      clearTimeout(timer);
      worker.terminate();
      if (event.data.error) reject(new Error(event.data.error));
      else resolve(event.data.content);
    };
    worker.onerror = (event) => {
      clearTimeout(timer);
      worker.terminate();
      reject(new Error(event.message));
    };
    worker.postMessage(content);
  });
}
