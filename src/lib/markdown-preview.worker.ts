import { previewSections } from './markdown-preview';

self.onmessage = (event: MessageEvent<{ id: number; markdown: string }>) => {
  try {
    self.postMessage({ id: event.data.id, sections: previewSections(event.data.markdown) });
  } catch (error) {
    self.postMessage({ id: event.data.id, error: String(error) });
  }
};
