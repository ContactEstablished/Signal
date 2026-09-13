interface Interval { id: string; start_min: number; end_min: number }
export interface Packed<T> { item: T; lane: number; lanes: number; top: number; height: number }
/** Layout uses painted extents, without changing logical schedule intervals. */
export function packLanes<T extends Interval>(items: readonly T[]): Packed<T>[] {
 const sorted = [...items].sort((a,b) => a.start_min-b.start_min || a.end_min-b.end_min || a.id.localeCompare(b.id));
 const result: Packed<T>[] = [];
 let group: Packed<T>[] = [], ends: number[] = [], groupEnd = -1;
 const finish = () => { for (const p of group) p.lanes = ends.length; group = []; ends = []; };
 for (const item of sorted) {
  if (item.start_min >= groupEnd) finish();
  const end = Math.min(1440, Math.max(item.end_min, item.start_min + 26));
  let lane = ends.findIndex(e => e <= item.start_min);
  if (lane < 0) lane = ends.length;
  ends[lane] = end;
  const p = { item, lane, lanes: 1, top: item.start_min, height: end-item.start_min };
  group.push(p); result.push(p); groupEnd = Math.max(...ends);
 }
 finish();
 return result;
}
