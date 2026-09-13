import { it,expect,vi,beforeEach } from 'vitest';
import { PlannerState } from '../../src/lib/state/planner.svelte';
import type { PlannerResult,PlannerSnapshot } from '../../src/lib/domain/planner';
const api=vi.hoisted(()=>({getPlanner:vi.fn(),applyPlanner:vi.fn()}));
vi.mock('../../src/lib/native/planner',()=>api);
const snapshot=(date='2025-09-12')=>({date,time_zone:'UTC',fingerprint:date,blocks:[],tasks:[],meetings:[],entries:[],claims:[],previous_blocks:[],copy_sources:[],dismissed:false,timers:{sessions:[],offset_ms:0,now_utc:'2025-09-12T00:00:00Z'}} as PlannerSnapshot);
const reply=(date='2025-09-12'):PlannerResult=>({snapshot:snapshot(date),outcome:{block_ids:['b']},changed_details:[],changed_entries:{}});
beforeEach(()=>vi.resetAllMocks());
it('rejects reads from an older date and from before a committed mutation',async()=>{
 const state=new PlannerState();state.initialize('2025-09-12','UTC');let resolve!:(s:PlannerSnapshot)=>void;
 api.getPlanner.mockImplementationOnce(()=>new Promise(r=>resolve=r));const old=state.load('UTC');state.date='2025-09-13';api.getPlanner.mockResolvedValue(snapshot('2025-09-13'));await state.load('UTC');resolve(snapshot());await old;expect(state.snapshot?.date).toBe('2025-09-13');
 api.getPlanner.mockImplementationOnce(()=>new Promise(r=>resolve=r));const stale=state.load('UTC');state.publish({...reply('2025-09-13'),snapshot:{...snapshot('2025-09-13'),fingerprint:'new'}});resolve(snapshot('2025-09-13'));await stale;expect(state.snapshot?.fingerprint).toBe('new');
});
it('retains exact unknown intent across date changes and read refreshes',async()=>{
 const state=new PlannerState();state.initialize('2025-09-12','UTC');state.reserve('create',{draft:{kind:'focus',task_id:null,start_min:540,end_min:600}});
 api.applyPlanner.mockRejectedValueOnce(new Error('Transport disconnected'));await expect(state.execute()).rejects.toThrow();const original=api.applyPlanner.mock.calls[0][0];expect(state.recovery).toBe(true);expect(()=>state.reserve('dismiss',{})).toThrow('Retry');
 state.date='2025-09-13';api.getPlanner.mockResolvedValue(snapshot('2025-09-13'));await state.load('UTC');expect(state.recovery).toBe(true);api.applyPlanner.mockResolvedValue(reply());await state.execute(true);expect(api.applyPlanner.mock.calls[1][0]).toEqual(original);expect(state.snapshot?.date).toBe('2025-09-13');expect(state.recovery).toBe(false);
});
it('known rolled-back validation permits a corrected operation with a new identity',async()=>{
 const state=new PlannerState();state.initialize('2025-09-12','UTC');state.reserve('dismiss',{});api.applyPlanner.mockRejectedValueOnce({code:'Conflict',message:'Changed'});await expect(state.execute()).rejects.toEqual({code:'Conflict',message:'Changed'});expect(state.recovery).toBe(false);state.reserve('dismiss',{});api.applyPlanner.mockResolvedValue(reply());await state.execute();expect(api.applyPlanner.mock.calls[0][0].requestId).not.toBe(api.applyPlanner.mock.calls[1][0].requestId);
});
