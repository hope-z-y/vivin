import type { Annotation, AnnotationKind } from './project-api';

export interface AnnotationMark {
  id: string;
  offset: number;
  length: number;
  kind: AnnotationKind;
  quote: string;
}

function matchingSuffix(left: string, right: string): number {
  let count = 0;
  while (
    count < left.length &&
    count < right.length &&
    left[left.length - count - 1] === right[right.length - count - 1]
  )
    count++;
  return count;
}

function matchingPrefix(left: string, right: string): number {
  let count = 0;
  while (count < left.length && count < right.length && left[count] === right[count]) count++;
  return count;
}

export function resolveAnnotation(content: string, annotation: Annotation): number | null {
  if (!annotation.quote) return null;
  const positions: number[] = [];
  let from = 0;
  while (from < content.length) {
    const found = content.indexOf(annotation.quote, from);
    if (found < 0) break;
    positions.push(found);
    from = found + Math.max(1, annotation.quote.length);
  }
  if (positions.length === 1) return positions[0] ?? null;
  if (!positions.length) return null;

  const ranked = positions
    .map((offset) => ({
      offset,
      score:
        matchingSuffix(
          annotation.contextBefore,
          content.slice(Math.max(0, offset - annotation.contextBefore.length), offset),
        ) +
        matchingPrefix(
          annotation.contextAfter,
          content.slice(
            offset + annotation.quote.length,
            offset + annotation.quote.length + annotation.contextAfter.length,
          ),
        ),
    }))
    .sort((a, b) => b.score - a.score);
  if (!ranked[0] || ranked[0].score < 4 || ranked[0].score === ranked[1]?.score) return null;
  return ranked[0].offset;
}

export function annotationMarks(content: string, annotations: Annotation[]): AnnotationMark[] {
  return annotations.flatMap((annotation) => {
    const offset = resolveAnnotation(content, annotation);
    return offset === null
      ? []
      : [
          {
            id: annotation.id,
            offset,
            length: annotation.quote.length,
            kind: annotation.kind,
            quote: annotation.quote,
          },
        ];
  });
}
