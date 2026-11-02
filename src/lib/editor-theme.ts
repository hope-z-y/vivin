import frameLight from '@milkdown/crepe/theme/frame.css?inline';
import frameDark from '@milkdown/crepe/theme/frame-dark.css?inline';
import classicLight from '@milkdown/crepe/theme/classic.css?inline';
import classicDark from '@milkdown/crepe/theme/classic-dark.css?inline';
import nordLight from '@milkdown/crepe/theme/nord.css?inline';
import nordDark from '@milkdown/crepe/theme/nord-dark.css?inline';
import '@milkdown/crepe/theme/common/style.css';

export type DocumentTheme = 'frame' | 'classic' | 'nord';
export const documentThemes: { value: DocumentTheme; label: string }[] = [
  { value: 'frame', label: 'Frame' },
  { value: 'classic', label: 'Crepe' },
  { value: 'nord', label: 'Nord' },
];

// Scope the complete official styles without changing their variables or rules.
// Theme family and appearance are independent; changing either only changes CSS.
export function installEditorThemes() {
  const themes = {
    frame: [frameLight, frameDark],
    classic: [classicLight, classicDark],
    nord: [nordLight, nordDark],
  };
  const style = document.createElement('style');
  style.dataset.vivinEditorThemes = '';
  style.textContent = Object.entries(themes)
    .flatMap(([theme, variants]) =>
      variants.map((css, index) =>
        css.replace(
          /\.milkdown/g,
          `.app-shell[data-document-theme="${theme}"]${index === 1 ? '.theme-dark' : ':not(.theme-dark)'} .milkdown`,
        ),
      ),
    )
    .join('\n');
  document.head.append(style);
}
