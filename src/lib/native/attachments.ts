import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Keep image paste scoped to the open editor, including when a text field has focus.
export function pasteAttachments(
  node: HTMLDialogElement,
  options: { enabled: () => boolean; stage: (image?: File) => void },
) {
  const paste = (event: ClipboardEvent) => {
    if (event.defaultPrevented || !options.enabled()) return;
    const clipboard = event.clipboardData;
    if (!clipboard) return;
    const files = Array.from(clipboard.files ?? []);
    if (!files.length) {
      for (const item of Array.from(clipboard.items ?? [])) {
        if (item.kind === 'file' && item.type.startsWith('image/')) {
          const file = item.getAsFile();
          if (file) files.push(file);
        }
      }
    }
    const image = files.find((file) => file.type.startsWith('image/'));
    if (image) {
      event.preventDefault();
      options.stage(image);
    } else if (!Array.from(clipboard.types ?? []).some((type) => type.startsWith('text/'))) {
      event.preventDefault();
      options.stage();
    }
  };
  const shortcut = (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.repeat || event.altKey || event.shiftKey ||
      !(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'v' || !options.enabled()) return;
    if (event.target instanceof Element && event.target.closest('input, textarea, [contenteditable]:not([contenteditable="false"])')) return;
    // WebView may not emit paste on a non-editable target for native bitmaps.
    event.preventDefault();
    options.stage();
  };
  node.addEventListener('paste', paste);
  node.addEventListener('keydown', shortcut);
  return { destroy() {
    node.removeEventListener('paste', paste);
    node.removeEventListener('keydown', shortcut);
  } };
}

export async function imageBytes(image?: File): Promise<number[] | undefined> {
  if (!image) return undefined;
  if (image.size > 10 * 1024 * 1024) throw new Error('Choose an image up to 10 MiB.');
  return Array.from(new Uint8Array(await image.arrayBuffer()));
}
export interface DropEditor {
  dropTarget: () => HTMLDialogElement | null;
  setDropFeedback: (active: boolean) => void;
  stage: (paths?: string[]) => Promise<void>;
}
export async function subscribeAttachments(
  editor: () => DropEditor | null,
  onError: (error: unknown) => void,
) {
  return getCurrentWebview().onDragDropEvent((event) => {
    const active = editor();
    if (!active) return;
    const payload = event.payload;
    if (payload.type === 'leave') {
      active.setDropFeedback(false);
      return;
    }
    void getCurrentWindow()
      .scaleFactor()
      .then((scale) => {
        if (editor() !== active) return;
        const target = active.dropTarget();
        const bounds = target?.getBoundingClientRect();
        const point = payload.position;
        const inside =
          !!bounds &&
          point.x / scale >= bounds.left &&
          point.x / scale <= bounds.right &&
          point.y / scale >= bounds.top &&
          point.y / scale <= bounds.bottom;
        active.setDropFeedback(inside && payload.type !== 'drop');
        if (inside && payload.type === 'drop')
          return active.stage(payload.paths);
      })
      .catch(onError);
  });
}
