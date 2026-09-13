import { invoke } from '@tauri-apps/api/core';
import type { PlannerQuery, PlannerSnapshot, PlannerInput, PlannerResult } from '../domain/planner';
export const getPlanner = (query: PlannerQuery) => invoke<PlannerSnapshot>('get_planner', { query });
export const applyPlanner = (input: PlannerInput) => invoke<PlannerResult>('apply_planner', { input });
