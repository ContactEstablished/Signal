import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
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
