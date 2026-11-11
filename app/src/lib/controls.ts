export const controlActions = [
  ...Array.from({length:9},(_,value)=>({id:`clue-${value}`,label:value===0?'0 · Revealed blank':`${value} · Revealed number`,keys:[String(value)],value})),
  {id:'flag',label:'Toggle flag',keys:['f'],value:-2},
  {id:'covered',label:'Covered',keys:['h'],value:-1},
  {id:'safe',label:'Safe · clue unknown',keys:['s'],value:-4},
  {id:'unreadable',label:'Unreadable',keys:['?'],value:-5},
  {id:'left',label:'Move left',keys:['ArrowLeft']},
  {id:'right',label:'Move right',keys:['ArrowRight']},
  {id:'up',label:'Move up',keys:['ArrowUp']},
  {id:'down',label:'Move down',keys:['ArrowDown']},
  {id:'undo',label:'Undo',keys:['Ctrl+z','Meta+z']},
  {id:'redo',label:'Redo',keys:['Ctrl+Shift+z','Ctrl+y','Meta+Shift+z']}
];
export type Bindings = Record<string,string[]>;
export function defaultBindings():Bindings{return Object.fromEntries(controlActions.map(action=>[action.id,[...action.keys]]));}
// Printable symbols already include their Shift variant (e.g. ?); letters keep Shift explicit.
export function keyBinding(event:Pick<KeyboardEvent,'key'|'ctrlKey'|'metaKey'|'altKey'|'shiftKey'>):string|null{
  if(['Control','Meta','Alt','Shift','Dead','Unidentified','Process','AltGraph'].includes(event.key))return null;
  const key=event.key.length===1?event.key.toLowerCase():event.key;
  const shift=event.shiftKey&&(key.length>1||/^[a-z]$/.test(key));
  return [event.ctrlKey?'Ctrl':null,event.metaKey?'Meta':null,event.altKey?'Alt':null,shift?'Shift':null,key===' '?'Space':key].filter(Boolean).join('+');
}
export function reservedBinding(binding:string):boolean{
  return ['Escape','Tab','Shift+Tab','Alt+F4','Shift+Insert','Ctrl+Insert','Shift+Delete'].includes(binding)
    || /^(Ctrl|Meta)\+(Shift\+)?[acvxlwtrnq]$/.test(binding)
    || /^(Ctrl|Meta)\+([+=\-0]|F4)$/.test(binding);
}
export function bindingLabel(binding:string):string{
  return binding.split('+').map(key=>({Meta:'Win / ⌘',ArrowLeft:'←',ArrowRight:'→',ArrowUp:'↑',ArrowDown:'↓'}[key]??(key.length===1?key.toUpperCase():key))).join(' + ');
}
export function bindingOwner(bindings:Bindings,key:string):string|undefined{return controlActions.find(action=>bindings[action.id]?.includes(key))?.id;}
