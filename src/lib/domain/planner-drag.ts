import type { PlannerTask } from './planner';

export const SIDEBAR_BLOCK_MINUTES = 60;
export type PlannerTaskDrag = Pick<PlannerTask, 'id' | 'title' | 'project_color'> & {
  x: number;
  y: number;
};
