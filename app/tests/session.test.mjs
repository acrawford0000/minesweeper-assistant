import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readSession } from '../src/lib/types.ts';
const session = {version:1,board:{rows:2,cols:2,cells:[0,-1,-2,-4],totalMines:1,trustFlags:false,revision:2},image:null,scan:null,reviewed:true,name:'Example'};
test('sessions preserve semantic cell states and clear the old analysis revision',()=>{
  const loaded=readSession(JSON.stringify(session));assert.deepEqual(loaded.board.cells,session.board.cells);assert.equal(loaded.board.revision,0);
});
test('rejects malformed cell counts, mine counts and states',()=>{
  for(const board of [{...session.board,cells:[-1]},{...session.board,totalMines:5},{...session.board,cells:[0,-8,0,0]},{...session.board,rows:NaN}]) assert.throws(()=>readSession(JSON.stringify({...session,board})));
});
test('rejects external image references',()=>assert.throws(()=>readSession(JSON.stringify({...session,image:'https://example.com/image.png'}))));
test('old confirmed-mine entries migrate to the single trusted flag state',()=>{
 const loaded=readSession(JSON.stringify({...session,board:{...session.board,cells:[0,-1,-3,-4],trustFlags:false}}));
 assert.equal(loaded.board.cells[2],-2);assert.equal(loaded.board.trustFlags,true);
});
test('accepted scan appearance and corrections survive a session round trip',()=>{
 const approvedScan={board:{...session.board,cells:[6,7,-2,0]},rect:{x:12,y:30,width:60,height:60},signatures:Array(4).fill('A'.repeat(260)),data:null};
 assert.deepEqual(readSession(JSON.stringify({...session,approvedScan})).approvedScan,approvedScan);
 assert.equal(readSession(JSON.stringify(session)).approvedScan,null);
 for(const bad of [{...approvedScan,signatures:['short']},{...approvedScan,board:{...approvedScan.board,totalMines:99}},{...approvedScan,rect:{...approvedScan.rect,x:-1}}])assert.equal(readSession(JSON.stringify({...session,approvedScan:bad})).approvedScan,null);
});
