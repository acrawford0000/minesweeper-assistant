import {test} from 'node:test';
import assert from 'node:assert/strict';
import {defaultBindings,keyBinding,bindingOwner,reservedBinding} from '../src/lib/controls.ts';
const key=(value,extra={})=>keyBinding({key:value,ctrlKey:false,metaKey:false,altKey:false,shiftKey:false,...extra});
test('printable symbol bindings match actual shifted keyboard events',()=>{
  assert.equal(key('?',{shiftKey:true}),'?');
  assert.equal(bindingOwner(defaultBindings(),key('?',{shiftKey:true})),'unreadable');
  assert.equal(key('Z',{ctrlKey:true,shiftKey:true}),'Ctrl+Shift+z');
  assert.equal(key('Shift',{shiftKey:true}),null);
});
test('multiple keys preserve defaults and separate fresh settings',()=>{
  const settings=defaultBindings();settings['clue-0'].push('q');
  assert.equal(bindingOwner(settings,key('q')),'clue-0');assert.equal(bindingOwner(settings,key('0')),'clue-0');
  assert.deepEqual(defaultBindings()['clue-0'],['0']);
  assert.equal(bindingOwner(settings,key('0',{ctrlKey:true})),undefined);
});
test('capture reserves clipboard, focus and close shortcuts but allows nearby letter keys',()=>{
  for(const value of ['Ctrl+v','Meta+v','Ctrl+Shift+v','Escape','Tab','Shift+Tab','Alt+F4'])assert.ok(reservedBinding(value),value);
  for(const value of ['q','w','e','?','Ctrl+e','Ctrl+z'])assert.equal(reservedBinding(value),false,value);
});
