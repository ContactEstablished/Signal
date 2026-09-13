import * as api from '../native/planner';
import { errorMessage } from '../domain/types';
import type { PlannerInput,PlannerQuery,PlannerSnapshot,PlannerResult } from '../domain/planner';
export class PlannerState {
 date=$state('');scope=$state('all');followToday=$state(true);snapshot=$state<PlannerSnapshot|null>(null);
 loading=$state(false);pending=$state(false);recovery=$state(false);error=$state('');draftOpen=$state(false);
 private serial=0;private epoch=0;private intent:PlannerInput|null=null;private zone='';
 get guarded(){return this.draftOpen||this.pending||this.recovery;}
 initialize(date:string,zone:string){if(!this.date)this.date=date;this.zone=zone;}
 invalidate(){this.epoch++;this.serial++;this.loading=false;}
 async load(zone:string){if(!this.date)return;this.zone=zone;const query={date:this.date,timeZone:zone},serial=++this.serial,epoch=this.epoch;this.loading=true;
  try{const result=await api.getPlanner(query);if(serial===this.serial&&epoch===this.epoch&&this.matches(query)){this.snapshot=result;if(!this.recovery)this.error='';}}
  catch(e){if(serial===this.serial)this.error=errorMessage(e);}
  finally{if(serial===this.serial)this.loading=false;}
 }
 private matches(q:PlannerQuery){return q.date===this.date&&q.timeZone===this.zone;}
 publish(result:PlannerResult){this.invalidate();if(this.matches({date:result.snapshot.date,timeZone:result.snapshot.time_zone}))this.snapshot=result.snapshot;}
 assertWritable(){if(this.recovery)throw new Error('Resolve the pending planner operation with Retry before making another change.');}
 reserve(action:PlannerInput['action'],payload:Record<string,unknown>,fingerprint?:string){
  this.assertWritable();if(this.pending)throw new Error('A planner operation is already pending.');
  this.intent=JSON.parse(JSON.stringify({requestId:crypto.randomUUID(),query:{date:this.date,timeZone:this.zone},action,payload,...(fingerprint?{expectedFingerprint:fingerprint}:{})}));this.pending=true;this.error='';
 }
 cancelReserved(error:unknown){if(!this.recovery){this.intent=null;this.pending=false;this.error=errorMessage(error);}}
 async execute(retry=false):Promise<PlannerResult>{
  if(!this.intent||(!this.pending&&!retry))throw new Error('No planner operation to retry.');
  this.pending=true;this.error='';
  try{const result=await api.applyPlanner(this.intent);this.publish(result);this.intent=null;this.recovery=false;return result;}
  catch(e){const known=!!e&&typeof e==='object'&&'code' in e&&['Validation','NotFound','Conflict','Database'].includes(String(e.code));this.recovery=!known;if(known)this.intent=null;this.error=known?errorMessage(e):'The result is unknown. Retry to check this exact planner operation safely.';throw e;}
  finally{this.pending=false;}
 }
}
