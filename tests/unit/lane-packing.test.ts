import { describe,it,expect } from 'vitest';
import { packLanes } from '../../src/lib/domain/lane-packing';
const item=(id:string,start_min:number,end_min:number)=>({id,start_min,end_min});
describe('painted planner lanes',()=>{
 it('separates short adjacent cards without changing their scheduled range',()=>{
  const input=[item('a',540,555),item('b',555,570),item('c',600,630)];
  const packed=packLanes(input);
  expect(packed.map(p=>[p.lane,p.lanes,p.height])).toEqual([[0,2,26],[1,2,26],[0,1,30]]);
  expect(input[0].end_min).toBe(555);
 });
 it('handles connected nested overlaps and resets independent groups',()=>{
  const packed=packLanes([item('e',700,730),item('d',620,660),item('b',550,600),item('a',540,660),item('c',570,620)]);
  expect(packed.map(p=>[p.item.id,p.lane,p.lanes])).toEqual([['a',0,3],['b',1,3],['c',2,3],['d',1,3],['e',0,1]]);
 });
 it('breaks ties deterministically at four lanes and clips paint at midnight',()=>{
  const items=['d','a','c','b'].map(id=>item(id,540,600));
  expect(packLanes(items).map(p=>[p.item.id,p.lane,p.lanes])).toEqual([['a',0,4],['b',1,4],['c',2,4],['d',3,4]]);
  expect(packLanes([item('late',1425,1440)])[0].height).toBe(15);
 });
});
