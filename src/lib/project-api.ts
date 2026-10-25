import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export interface ProjectInfo {
  name: string;
  path: string;
}

export interface TreeNode {
  name: string;
  path: string;
  kind: 'directory' | 'document';
  children: TreeNode[];
  pinned: boolean;
  lockSource: string | null;
}

export interface ResourceNode {
  name: string;
  path: string;
  localPath: string;
  kind: 'directory' | 'file';
  fileType: string;
  children: ResourceNode[];
}

export interface SearchHit {
  path: string;
  offset: number;
  excerpt: string;
}

export interface Term {
  id: string;
  name: string;
  definition: string;
}

export interface TermException {
  id: string;
  document: string;
  termId: string;
  offset: number;
  contextBefore: string;
  contextAfter: string;
}

export type AnnotationKind = 'todo' | 'comment' | 'question' | 'error';

export interface Annotation {
  id: string;
  document: string;
  kind: AnnotationKind;
  quote: string;
  contextBefore: string;
  contextAfter: string;
  offset: number;
  note: string;
}

export interface ProjectData {
  schemaVersion: number;
  pinned: string[];
  locked: string[];
  terms: Term[];
  termExceptions: TermException[];
  annotations: Annotation[];
}

export interface HistoryEntry {
  id: string;
  time: number;
  operation: string;
  target: string;
  beforeContent: string | null;
  afterContent: string | null;
  beforeData: ProjectData | null;
  afterData: ProjectData | null;
}

export interface BridgeInfo {
  url: string;
  configPath: string;
}

export interface Document {
  path: string;
  content: string;
  revision: string;
}

export function nativeAvailable(): boolean {
  return isTauri();
}

export async function chooseProject(locale: 'zh' | 'en' = 'zh'): Promise<ProjectInfo | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: locale === 'en' ? 'Open vivin project' : '打开 vivin 项目',
  });
  if (!selected || typeof selected !== 'string') return null;
  return invoke<ProjectInfo>('open_project', { path: selected });
}

export function openProject(path: string): Promise<ProjectInfo> {
  return invoke<ProjectInfo>('open_project', { path });
}

export async function createProject(
  name: string,
  locale: 'zh' | 'en' = 'zh',
): Promise<ProjectInfo | null> {
  const parent = await open({
    directory: true,
    multiple: false,
    title: locale === 'en' ? 'Choose a location for the new project' : '选择新项目的存放位置',
  });
  if (!parent || typeof parent !== 'string') return null;
  return invoke<ProjectInfo>('create_project', { parent, name });
}

export function listProjectTree(): Promise<TreeNode[]> {
  return invoke<TreeNode[]>('list_project_tree');
}

export function readDocument(path: string): Promise<Document> {
  return invoke<Document>('read_document', { path });
}

export function saveDocument(document: Document): Promise<Document> {
  return invoke<Document>('save_document', {
    path: document.path,
    content: document.content,
    expectedRevision: document.revision,
  });
}

export function createDocument(directory: string, name: string): Promise<Document> {
  return invoke<Document>('create_document', { directory, name });
}

export function createDirectory(directory: string, name: string): Promise<string> {
  return invoke<string>('create_directory', { directory, name });
}

export function moveEntry(
  path: string,
  destinationDirectory: string,
  name: string,
): Promise<string> {
  return invoke<string>('move_entry', { path, destinationDirectory, name });
}

export function deleteEntry(path: string): Promise<void> {
  return invoke<void>('delete_entry', { path });
}

export function setPinned(path: string, pinned: boolean): Promise<void> {
  return invoke<void>('set_pinned', { path, pinned });
}

export function setLocked(path: string, locked: boolean): Promise<void> {
  return invoke<void>('set_locked', { path, locked });
}

export function getProjectData(): Promise<ProjectData> {
  return invoke<ProjectData>('get_project_data');
}

export function upsertTerm(id: string | null, name: string, definition: string): Promise<Term> {
  return invoke<Term>('upsert_term', { id, name, definition });
}

export function deleteTerm(id: string): Promise<void> {
  return invoke<void>('delete_term', { id });
}

export function setTermException(
  document: string,
  termId: string,
  offset: number,
  disabled: boolean,
  exceptionId?: string,
): Promise<TermException | null> {
  return invoke<TermException | null>('set_term_exception', {
    document,
    termId,
    offset,
    disabled,
    exceptionId: exceptionId ?? null,
  });
}

export function upsertAnnotation(annotation: Annotation): Promise<Annotation> {
  return invoke<Annotation>('upsert_annotation', { annotation });
}

export function deleteAnnotation(id: string): Promise<void> {
  return invoke<void>('delete_annotation', { id });
}

export function searchDocuments(query: string): Promise<SearchHit[]> {
  return invoke<SearchHit[]>('search_documents', { query });
}

export function listResources(): Promise<ResourceNode[]> {
  return invoke<ResourceNode[]>('list_resources');
}

export async function importResource(directory = ''): Promise<string | null> {
  const source = await open({ multiple: false, title: '选择要加入资源库的文件' });
  if (!source || typeof source !== 'string') return null;
  return invoke<string>('import_resource', { source, directory });
}

export async function importImage(document: string, file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (bytes.length > 20_000_000) throw new Error('图片不能超过 20 MB');
  let binary = '';
  for (let from = 0; from < bytes.length; from += 8192)
    binary += String.fromCharCode(...bytes.subarray(from, from + 8192));
  return invoke<string>('import_image', {
    document,
    name: file.name || 'image.png',
    bytesBase64: btoa(binary),
  });
}

export function readProjectImage(document: string, source: string): Promise<string> {
  return invoke<string>('read_project_image', { document, source });
}

export function readProjectRules(): Promise<Document> {
  return invoke<Document>('read_project_rules');
}

export function saveProjectRules(document: Document): Promise<Document> {
  return invoke<Document>('save_project_rules', {
    content: document.content,
    expectedRevision: document.revision,
  });
}

export function listHistory(): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>('list_history');
}

export function undoChange(id: string): Promise<HistoryEntry> {
  return invoke<HistoryEntry>('undo_change', { id });
}

export function bridgeInfo(): Promise<BridgeInfo> {
  return invoke<BridgeInfo>('bridge_info');
}

export function setDirtyDraft(dirty: boolean): Promise<void> {
  return invoke<void>('set_dirty_draft', { dirty });
}

export function publishDraft(
  path: string,
  content: string,
  baseRevision: string,
  sequence: number,
): Promise<string> {
  return invoke<string>('publish_draft', { path, content, baseRevision, sequence });
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
