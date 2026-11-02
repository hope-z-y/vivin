import { formatMarkdown } from './markdown-format';

self.onmessage = (event: MessageEvent<string>) => {
  try {
    self.postMessage({ content: formatMarkdown(event.data) });
  } catch (error) {
    self.postMessage({ error: String(error) });
  }
};
