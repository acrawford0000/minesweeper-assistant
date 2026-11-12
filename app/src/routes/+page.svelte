<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {listen,type UnlistenFn} from '@tauri-apps/api/event';
  import { readSession, type Board, type Analysis, type Scan, type Session, type Rect, type BoardComparison, type ReviewReference } from '$lib/types';
  import Settings from '$lib/Settings.svelte';
  import {controlActions,defaultBindings,keyBinding,bindingLabel,bindingOwner} from '$lib/controls';
  import {fitCellText} from '$lib/boardText';
  import type {CaptureStatus,CapturedRegion} from '$lib/types';

  let board = $state<Board>({rows:9,cols:9,cells:Array(81).fill(-1),totalMines:10,trustFlags:true,revision:0});
  let analysis = $state<Analysis|null>(null);
  let image = $state<string|null>(null);
  let scan = $state<Scan|null>(null);
  let reviewed = $state(true);
  let approvedScan = $state<ReviewReference|null>(null);
  function rememberApproval(){if(scan)approvedScan={board:{...board,cells:[...board.cells]},rect:{...scan.rect},signatures:scan.signatures??null,data:scan.signatures?null:image};}
  let name = $state('Sample board');
  let selected = $state<number|null>(null);
  let busy = $state(false);
  let scanning = $state(false);
  let ready = $state(false);
  let error = $state('');
  let notice = $state('');
  let notification = $state<{message:string;kind:'success'|'error';source:string}|null>(null);
  let notificationTimer:ReturnType<typeof setTimeout>;
  function notify(message:string,kind:'success'|'error',source:string){clearTimeout(notificationTimer);notification={message,kind,source};notificationTimer=setTimeout(()=>notification=null,kind==='error'?12000:4500);}
  function dismissNotification(){clearTimeout(notificationTimer);notification=null;}
  let showHelp = $state(false);
  let showSettings = $state(false);
  let bindings = $state(defaultBindings());
  let boardFontScale = $state(100);
  let showNew = $state(false);
  let showGrid = $state(false);
  let showSource = $state(true);
  let newRows = $state<number|undefined>(9);
  let newCols = $state<number|undefined>(9);
  let newMines = $state<number|undefined>(10);
  let gridRows = $state<number|undefined>();
  let gridCols = $state<number|undefined>();
  let useGrid = $state(false);
  let crop = $state<Rect|null>(null);
  let cropStart = $state<{x:number;y:number}|null>(null);
  let cropEnd = $state<{x:number;y:number}|null>(null);
  let imageWidth = $state(0);
  let imageHeight = $state(0);
  let imageElement = $state<HTMLImageElement>();
  let fileInput: HTMLInputElement;
  let sessionInput: HTMLInputElement;
  let zoom = $state(1);
  let panning = $state(false);
  let zoomRequest=0;
  let overlay = $state(true);
  let showSafe = $state(false);
  let efficiencyMode = $state(false);
  let activeStep = $state(0);
  let activeTool = $state<number|null>(null);
  let history = $state<Session[]>([]);
  let future = $state<Session[]>([]);
  let pendingImage = $state<string|null>(null);
  let counterValue = $state<number|undefined>();
  let showCounter = $state(false);
  let useScannedCounter = $state(true);
  let timer:ReturnType<typeof setTimeout>;
  let requestId=0;
  let captureStatus=$state<CaptureStatus>({region:null,hotkey:null,configuredHotkey:'y',hotkeyEnabled:false,shortcutError:null,selecting:false});
  let hotkeySaving=$state(false);
  async function toggleCaptureHotkey(enabled:boolean){if(hotkeySaving)return;hotkeySaving=true;try{captureStatus=await invoke<CaptureStatus>('set_capture_hotkey_enabled',{enabled});}catch(cause){error=String(cause);}finally{hotkeySaving=false;}}
  let capturing=$state(false);
  let choosingCapture=$state(false);
  let captureRequestRevision:number|null=null;
  async function chooseCaptureArea(){if(scanning||capturing||choosingCapture)return;choosingCapture=true;captureRequestRevision=board.revision;try{await invoke('select_capture_area');}catch(cause){choosingCapture=false;error=String(cause);}}
  async function applyCaptured(captured:CapturedRegion,revision:number){
    captureStatus={...captureStatus,region:captured.region,selecting:false};
    if(board.revision!==revision){notice='The board changed during capture. Capture again to refresh it.';return;}
    remember();image=captured.image;pendingImage=null;scan=null;crop=null;cropStart=null;cropEnd=null;gridRows=undefined;gridCols=undefined;useGrid=false;reviewed=false;name='Screen capture';board={...board,revision:board.revision+1};
    await tick();await scanScreenshot();
  }
  async function captureNow(){if(scanning||capturing||choosingCapture||showSettings||showNew||showHelp)return;if(!captureStatus.region){notice='Choose a capture area around the board and mine counter first.';return;}const revision=board.revision;capturing=true;try{const captured=await invoke<CapturedRegion>('capture_region');await applyCaptured(captured,revision);}catch(cause){error=String(cause);}finally{capturing=false;}}
  function showBest(){if(best===null)return;selected=best;document.getElementById(`cell-${best}`)?.scrollIntoView({block:'nearest',inline:'nearest'});}

  function focusModal(node:HTMLElement){
    const previous=document.activeElement as HTMLElement|null;
    const first=node.querySelector<HTMLElement>('input,button');(first??node).focus();
    const trap=(event:KeyboardEvent)=>{
      if(event.key==='Escape'){event.stopPropagation();showNew=false;showHelp=false;showSettings=false;pendingImage=null;return;}
      if(event.key!=='Tab')return;
      const elements=Array.from(node.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),[tabindex="0"]'));
      const start=elements[0],end=elements.at(-1);
      if(event.shiftKey&&document.activeElement===start){event.preventDefault();end?.focus();}
      else if(!event.shiftKey&&document.activeElement===end){event.preventDefault();start?.focus();}
    };
    node.addEventListener('keydown',trap);
    return {destroy(){node.removeEventListener('keydown',trap);previous?.focus();}};
  }

  const labels:Record<number,string> = {'-1':'Covered','-2':'Flagged mine','-3':'Flagged mine','-4':'Safe, clue not entered','-5':'Unreadable'};
  const tools=[{value:0,label:'0',title:'Revealed blank (zero)'},...Array.from({length:8},(_,i)=>({value:i+1,label:String(i+1),title:`Revealed ${i+1}`})),{value:-1,label:'□',title:'Covered'},{value:-2,label:'⚑',title:'Flag'},{value:-4,label:'✓',title:'Safe, clue unknown'},{value:-5,label:'?',title:'Unreadable'}];
  function toolTitle(tool:{value:number;title:string}){const action=controlActions.find(a=>'value' in a&&a.value===tool.value);const keys=action?bindings[action.id]:[];return `${tool.title}${keys.length?` (${keys.map(bindingLabel).join(', ')})`:''}`;}
  const safeCount=$derived(analysis?.odds.filter((o,i)=>o.kind==='safe' && board.cells[i]<0 && board.cells[i]!==-4).length??0);
  const mineCount=$derived(analysis?.odds.filter((o,i)=>o.kind==='mine' && board.cells[i]<0 && board.cells[i]!==-3).length??0);
  const flags=$derived(board.cells.filter(v=>v===-2).length);
  const needsReview=$derived(board.cells.filter((value,i)=>value===-5||((value>0||value===-2)&&(scan?.confidence[i]??1)<0.8)).length);
  const unresolved=$derived(board.cells.filter(v=>v===-5).length);
  const validAnalysis=$derived(analysis && !capturing && !scanning && !['invalid','review'].includes(analysis.status)?analysis:null);
  const plan=$derived(validAnalysis?.efficiency??null);
  const planTargets=$derived(new Set(plan?.targets??[]));
  const best=$derived(validAnalysis?.recommendations[0]??null);
  const selectedOdds=$derived(selected!==null?validAnalysis?.odds[selected]:null);
  const progress=$derived(board.cells.filter(v=>v>=0).length);
  // Replacing the board during an edit must not reinitialize the user's view.
  const boardDimensions=$derived(`${board.rows}x${board.cols}`);
  const cropStyle=$derived(cropStart&&cropEnd?{x:Math.min(cropStart.x,cropEnd.x),y:Math.min(cropStart.y,cropEnd.y),width:Math.abs(cropEnd.x-cropStart.x),height:Math.abs(cropEnd.y-cropStart.y)}:null);

  function snapshot():Session {return {version:1,board:{...board,cells:[...board.cells]},image,scan,reviewed,name,approvedScan};}
  function saveLocal(){try {const s=snapshot();if(s.image && s.image.length>3_000_000)s.image=null;if(s.approvedScan?.data&&s.approvedScan.data.length>3_000_000)s.approvedScan={...s.approvedScan,data:null};localStorage.setItem('minesweeper-assistant-v1',JSON.stringify(s));}catch {notice='Session is too large for autosave. Use Save session to keep a copy.';}}
  function remember(){history=[...history.slice(-59),snapshot()];future=[];}
  function apply(s:Session){const revision=board.revision+1;board={...s.board,cells:[...s.board.cells],revision};image=s.image;scan=s.scan;reviewed=s.reviewed;approvedScan=s.approvedScan??(s.reviewed&&s.scan?{board:{...s.board,cells:s.board.cells.map((v,i)=>s.scan?.cells[i]===-1?-1:v)},rect:s.scan.rect,signatures:s.scan.signatures??null,data:s.scan.signatures?null:s.image}:null);name=s.name;selected=null;crop=null;cropStart=null;cropEnd=null;error='';requestAnimationFrame(()=>{document.querySelector('.left-sidebar')?.scrollTo({top:0});document.querySelector('.right-sidebar')?.scrollTo({top:0});document.querySelector('.board-scroll')?.scrollTo({top:0,left:0});});}
  function undo(){if(!history.length)return;future=[...future,snapshot()];const previous=history.at(-1)!;history=history.slice(0,-1);apply(previous);}
  function redo(){if(!future.length)return;history=[...history,snapshot()];const next=future.at(-1)!;future=future.slice(0,-1);apply(next);}
  function editCell(value:number){if(selected===null)return;remember();board={...board,cells:board.cells.map((v,i)=>i===selected?value:v),revision:board.revision+1};error='';if(scan){const confidence=[...scan.confidence];confidence[selected]=1;const notes=[...scan.notes];notes[selected]='Manually corrected.';scan={...scan,confidence,notes};if(reviewed&&approvedScan&&scan.cells[selected]!==-1&&scan.cells[selected]!==0)approvedScan={...approvedScan,board:{...approvedScan.board,cells:approvedScan.board.cells.map((v,i)=>i===selected?value:v)}};}}
  function selectCell(index:number){if(panning)return;selected=index;if(activeTool!==null)editCell(activeTool);}
  function changeMines(value:number|null){remember();board={...board,totalMines:value,revision:board.revision+1};}
  async function useCounter(remaining:number){try{const total=await invoke<number>('mine_total_from_counter',{board:$state.snapshot(board),remaining});changeMines(total);showCounter=false;}catch(cause){error=String(cause);}}
  function coord(i:number){return `R${Math.floor(i/board.cols)+1} C${i%board.cols+1}`;}
  function fitBoard(){const viewport=document.querySelector('.board-scroll');if(!viewport)return;const size=Math.min((viewport.clientWidth-75)/board.cols-3,(viewport.clientHeight-80)/board.rows-3);zoom=Math.max(0.25,Math.min(1.2,size/43));}
  function initialView(){fitBoard();zoom=Math.max(0.65,zoom);}
  function isUncertain(i:number){const value=board.cells[i];return value===-5||((value>0||value===-2)&&((scan?.confidence[i]??1)<0.8||scan?.reviewCells?.includes(i)));}
  function clueColor(value:number){return scan?.clueColors?.[value]??['','#7cc7ff','#66c266','#ff7788','#ee88ff','#ddaa22','#7bc8d8','#d8e0e8','#a6b2c3'][value];}
  function stepForCell(i:number){if(!efficiencyMode||!plan)return null;const index=plan.actions.findIndex(action=>action.cell===i);return index<0?null:{number:index+1,kind:plan.actions[index].kind};}
  function showStep(index:number){if(!plan?.actions[index])return;activeStep=index;selected=plan.actions[index].cell;document.getElementById(`cell-${selected}`)?.scrollIntoView({block:'nearest',inline:'nearest'});}
  async function zoomAt(next:number,clientX?:number,clientY?:number){
    const viewport=document.querySelector<HTMLElement>('.board-scroll');const grid=document.querySelector<HTMLElement>('.board-grid');if(!viewport||!grid)return;
    const bounds=viewport.getBoundingClientRect();const before=grid.getBoundingClientRect();
    const x=clientX??bounds.left+viewport.clientWidth/2;const y=clientY??bounds.top+viewport.clientHeight/2;
    const ax=(x-before.left)/before.width;const ay=(y-before.top)/before.height;
    const request=++zoomRequest;zoom=Math.max(0.25,Math.min(3,next));await tick();if(request!==zoomRequest)return;
    const after=grid.getBoundingClientRect();viewport.scrollLeft+=after.left+ax*after.width-x;viewport.scrollTop+=after.top+ay*after.height-y;
  }
  function navigateBoard(node:HTMLElement){
    let drag:{id:number;x:number;y:number;left:number;top:number}|null=null;let suppressClick=false;
    const wheel=(event:WheelEvent)=>{event.preventDefault();const unit=event.deltaMode===1?16:event.deltaMode===2?node.clientHeight:1;void zoomAt(zoom*Math.exp(-event.deltaY*unit*0.0015),event.clientX,event.clientY);};
    const down=(event:PointerEvent)=>{
      if((event.ctrlKey&&event.button===0)||event.button===1){event.preventDefault();event.stopPropagation();suppressClick=true;panning=true;drag={id:event.pointerId,x:event.clientX,y:event.clientY,left:node.scrollLeft,top:node.scrollTop};node.setPointerCapture(event.pointerId);}
      else suppressClick=false;
    };
    const move=(event:PointerEvent)=>{if(!drag||event.pointerId!==drag.id)return;event.preventDefault();node.scrollLeft=drag.left-(event.clientX-drag.x);node.scrollTop=drag.top-(event.clientY-drag.y);};
    const up=(event:PointerEvent)=>{if(!drag||event.pointerId!==drag.id)return;drag=null;panning=false;if(node.hasPointerCapture(event.pointerId))node.releasePointerCapture(event.pointerId);};
    const click=(event:MouseEvent)=>{if(suppressClick||event.ctrlKey){event.preventDefault();event.stopImmediatePropagation();suppressClick=false;}};
    node.addEventListener('wheel',wheel,{passive:false});node.addEventListener('pointerdown',down,true);node.addEventListener('pointermove',move);node.addEventListener('pointerup',up);node.addEventListener('pointercancel',up);node.addEventListener('click',click,true);
    return{destroy(){node.removeEventListener('wheel',wheel);node.removeEventListener('pointerdown',down,true);node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',up);node.removeEventListener('pointercancel',up);node.removeEventListener('click',click,true);}};
  }
  function percent(prob:number|null|undefined,_kind?:string,asSafe=showSafe){if(prob===null||prob===undefined)return '—';return `${Math.round((asSafe?1-prob:prob)*100)}%`;}
  function riskTitle(i:number):string{if(efficiencyMode){const step=stepForCell(i);return `${coord(i)} · ${step?`Step ${step.number}: ${step.kind}${plan?.status==='guess'?' (guess)':''}`:planTargets.has(i)?'Expected to open in this sequence':board.cells[i]>=0?`Revealed ${board.cells[i]}`:labels[board.cells[i]]}`;}const odds=validAnalysis?.odds[i];if(!odds||odds.probability===null)return describe(i);const certainty=odds.kind==='safe'?'Proven safe':odds.kind==='mine'?'Guaranteed mine':'Rounded percentage; still a guess';const emphasis=odds.display?.highlight==='best'?' · Lowest calculated mine risk':odds.display?.highlight==='worst'?' · Highest calculated mine risk':odds.display?.muted?' · Common odds':'';return `${coord(i)} · ${percent(odds.probability,odds.kind,false)} mine risk · ${certainty}${emphasis}`;}
  function describe(i:number):string{if(efficiencyMode)return riskTitle(i);const state=board.cells[i];const odds=validAnalysis?.odds[i];return `${coord(i)}, ${state>=0?`revealed ${state}`:labels[state]}${odds?.probability!==null&&odds?.probability!==undefined?`, ${percent(odds.probability,odds.kind)} ${showSafe?'safe chance':'mine risk'}, ${odds.kind==='safe'?'proven safe':odds.kind==='mine'?'guaranteed mine':'rounded, not guaranteed'}`:''}`;}

  async function runAnalysis(revision:number,id:number){
    if(!reviewed || !ready)return;
    busy=true;
    try{const result=await invoke<Analysis>('analyze',{board:$state.snapshot(board)});if(id===requestId && revision===board.revision)analysis=result;}
    catch(cause){if(id===requestId)error=String(cause);}
    finally{if(id===requestId)busy=false;}
  }
  $effect(()=>{
    const revision=board.revision;const accepted=reviewed;const mounted=ready;
    if(!mounted)return;
    analysis=null;error='';requestId++;const id=requestId;
    clearTimeout(timer);void invoke('cancel_analysis').catch(()=>{});
    saveLocal();
    if(accepted){busy=true;timer=setTimeout(()=>void runAnalysis(revision,id),80);}else busy=false;
    return ()=>clearTimeout(timer);
  });
  $effect(()=>{if(ready)localStorage.setItem('minesweeper-efficiency-mode',efficiencyMode?'1':'0');});
  $effect(()=>{if(efficiencyMode){activeTool=null;activeStep=0;}});
  $effect(()=>{if(analysis?.revision!==undefined)activeStep=0;});
  $effect(()=>{if(notice)notify(notice,'success','notice');});
  $effect(()=>{if(error)notify(error,'error','error');});
  $effect(()=>{if(analysis?.status==='invalid')notify(analysis.message,'error','analysis');else if(analysis&&notification?.source==='analysis')dismissNotification();});

  async function demo(){remember();try{const b=await invoke<Board>('demo_board');apply({version:1,board:b,image:null,scan:null,reviewed:true,name:'Sample board'});notice='Sample position loaded. Import your own screenshot or create a blank board.';}catch(cause){error=String(cause);}}
  function createBoard(){if(!Number.isInteger(newRows)||!Number.isInteger(newCols)||newRows!<1||newCols!<1||newRows!>100||newCols!>100||newRows!*newCols!>2500|| (newMines!==undefined&&(!Number.isInteger(newMines)||newMines<0||newMines>newRows!*newCols!))){error='Enter valid dimensions (up to 2,500 cells) and a mine total that fits.';return;}remember();apply({version:1,board:{rows:newRows!,cols:newCols!,cells:Array(newRows!*newCols!).fill(-1),totalMines:newMines??null,trustFlags:true,revision:0},image:null,scan:null,reviewed:true,name:'Custom board'});showNew=false;}

  async function readImage(file:File){if(!['image/png','image/jpeg','image/webp'].includes(file.type)){error='Use a PNG, JPEG, or WebP screenshot.';return;}if(file.size>20_000_000){error='Use an image smaller than 20 MB.';return;}const reader=new FileReader();reader.onload=()=>{pendingImage=String(reader.result);};reader.onerror=()=>{error='Could not open this image.';};reader.readAsDataURL(file);}
  function acceptImage(){if(!pendingImage)return;remember();image=pendingImage;pendingImage=null;scan=null;crop=null;cropStart=null;cropEnd=null;gridRows=undefined;gridCols=undefined;useGrid=false;reviewed=false;name='Imported screenshot';board={...board,revision:board.revision+1};notice=efficiencyMode?'Screenshot received. Scanning and planning the next sequence…':'Screenshot loaded. Choose Scan screenshot, or drag around the board to crop first.';if(efficiencyMode)void tick().then(()=>scanScreenshot());}
  async function pasteImage(){try{const items=await navigator.clipboard.read();for(const item of items){const type=item.types.find(t=>t.startsWith('image/'));if(type){const blob=await item.getType(type);await readImage(new File([blob],'clipboard.png',{type:blob.type}));return;}}notice='No image found on the clipboard. Copy a screenshot and press Ctrl+V.';}catch{notice='Press Ctrl+V to paste your screenshot, or use Open image.';}}
  async function scanScreenshot(){
    if(!image)return;
    const startedRevision=board.revision;const previous=$state.snapshot(board);scanning=true;error='';
    try{
      const result=await invoke<Scan>('scan_image',{request:{data:image,crop,rows:useGrid?gridRows??null:null,cols:useGrid?gridCols??null:null,previous:approvedScan?$state.snapshot(approvedScan):null}});
      const accepted=result.autoAccept===true;
      const current:Board={rows:result.rows,cols:result.cols,cells:result.cells,totalMines:accepted?(result.counter?.totalMines??null):null,trustFlags:true,revision:startedRevision+1};
      const comparison=await invoke<BoardComparison>('compare_boards',{previous,current});
      if(board.revision!==startedRevision){notice='The board changed during scanning. Scan again to apply the screenshot.';return;}
      remember();scan={...result,comparison};useScannedCounter=!!result.counter&&result.counter.totalMines!==null;counterValue=result.counter?.value;gridRows=result.rows;gridCols=result.cols;board=current;reviewed=accepted;if(accepted)rememberApproval();
      selected=accepted?null:result.cells.findIndex((_,i)=>isUncertain(i));if(selected===-1)selected=null;
      notice=accepted?(comparison.kind==='progress'?'Board updated. '+comparison.newlyRevealed+' more squares revealed.':'Board ready.')+(board.totalMines===null?' Enter total mines for global odds.':' Using '+board.totalMines+' total mines.'):result.message;
    }catch(cause){error=String(cause);showGrid=true;}finally{scanning=false;}
  }
  async function acceptReview(){if(unresolved){error='Correct unreadable (?) cells before accepting the scan.';return;}try{let total=board.totalMines;if(useScannedCounter&&scan?.counter){total=await invoke<number>('mine_total_from_counter',{board:$state.snapshot(board),remaining:scan.counter.value});}remember();reviewed=true;board={...board,totalMines:total,trustFlags:true,revision:board.revision+1};rememberApproval();notice=total===null?'Scan accepted. Set total mines to calculate global probabilities.':`Scan accepted. Using ${total} total mines.`;}catch(cause){error=String(cause);}}
  function nextUncertain(){if(!scan)return;const start=selected??-1;for(let offset=1;offset<=board.cells.length;offset++){const i=(start+offset)%board.cells.length;if(isUncertain(i)){selected=i;document.getElementById(`cell-${i}`)?.scrollIntoView({block:'nearest',inline:'nearest'});return;}}notice='No ambiguous symbols need review. Blank cells are handled automatically.';}

  function imagePoint(event:PointerEvent){const bounds=imageElement!.getBoundingClientRect();return {x:Math.max(0,Math.min(1,(event.clientX-bounds.left)/bounds.width)),y:Math.max(0,Math.min(1,(event.clientY-bounds.top)/bounds.height))};}
  function cropDown(event:PointerEvent){if(!image)return;cropStart=imagePoint(event);cropEnd=cropStart;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);}
  function cropMove(event:PointerEvent){if((event.currentTarget as HTMLElement).hasPointerCapture(event.pointerId))cropEnd=imagePoint(event);}
  function cropUp(event:PointerEvent){const target=event.currentTarget as HTMLElement;if(!target.hasPointerCapture(event.pointerId))return;target.releasePointerCapture(event.pointerId);cropEnd=imagePoint(event);if(cropStyle && cropStyle.width>0.02&&cropStyle.height>0.02){crop={x:Math.floor(cropStyle.x*imageWidth),y:Math.floor(cropStyle.y*imageHeight),width:Math.floor(cropStyle.width*imageWidth),height:Math.floor(cropStyle.height*imageHeight)};notice='Crop selected. Scan screenshot to apply it.';}else {cropStart=null;cropEnd=null;crop=null;}}

  function downloadSession(){const blob=new Blob([JSON.stringify(snapshot(),null,2)],{type:'application/json'});const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download='minesweeper-session.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);notice='Session downloaded, including the original image when available.';}
  async function openSession(file:File){try{if(file.size>30_000_000)throw new Error('Session file is too large.');const s=readSession(await file.text());remember();apply(s);notice='Saved session opened.';}catch(cause){error=String(cause);}}

  onMount(()=>{
    let disposed=false;const unlisteners:UnlistenFn[]=[];
    const events=async()=>{try{
      for(const unlisten of await Promise.all([
        listen('capture-hotkey',()=>{if(captureStatus.hotkeyEnabled&&!hotkeySaving)void captureNow();}),
        listen<CapturedRegion>('capture-region-selected',event=>{choosingCapture=false;capturing=true;void applyCaptured(event.payload,captureRequestRevision??board.revision).catch(cause=>error=String(cause)).finally(()=>capturing=false);}),
        listen('capture-selection-cancelled',()=>{choosingCapture=false;notice='Capture area selection canceled. Your previous area is unchanged.';})
      ])){if(disposed)unlisten();else unlisteners.push(unlisten);}
      const status=await invoke<CaptureStatus>('capture_status');if(!disposed)captureStatus=status;
    }catch{/* Browser previews do not provide native screen capture. */}};void events();
    const init=async()=>{try{efficiencyMode=localStorage.getItem('minesweeper-efficiency-mode')==='1';const saved=localStorage.getItem('minesweeper-assistant-v1');if(saved)apply(readSession(saved));else{board=await invoke<Board>('demo_board');}}catch{notice='Import a screenshot or create a board. Rust analysis runs in the desktop app.';}ready=true;};void init();
    const paste=(event:ClipboardEvent)=>{if(showSettings||showNew||showHelp)return;const item=Array.from(event.clipboardData?.items??[]).find(i=>i.type.startsWith('image/'));if(item){event.preventDefault();const file=item.getAsFile();if(file)void readImage(file);}};
    const keyboard=(event:KeyboardEvent)=>{
      if(showSettings||showHelp||showNew||pendingImage||event.defaultPrevented||event.isComposing)return;
      const target=event.target as HTMLElement; if(target.closest('input,textarea,select,[contenteditable=true]'))return;
      if(event.key==='Escape'){activeTool=null;return;}
      const key=keyBinding(event);const id=key?bindingOwner(bindings,key):undefined;if(!id)return;
      if(id==='undo'||id==='redo'){event.preventDefault();if(!event.repeat){if(id==='undo')undo();else redo();}return;}
      if(selected===null)return;
      event.preventDefault();
      const action=controlActions.find(a=>a.id===id);
      if(action&&'value' in action){if(!event.repeat)editCell(id==='flag'&&board.cells[selected]===-2?-1:action.value!);return;}
      const row=Math.floor(selected/board.cols),col=selected%board.cols;
      if(id==='left')selected=row*board.cols+Math.max(0,col-1);
      else if(id==='right')selected=row*board.cols+Math.min(board.cols-1,col+1);
      else if(id==='up')selected=Math.max(0,row-1)*board.cols+col;
      else if(id==='down')selected=Math.min(board.rows-1,row+1)*board.cols+col;
      document.getElementById(`cell-${selected}`)?.focus();
    };
    window.addEventListener('paste',paste);window.addEventListener('keydown',keyboard);
    return ()=>{disposed=true;unlisteners.forEach(unlisten=>unlisten());window.removeEventListener('paste',paste);window.removeEventListener('keydown',keyboard);clearTimeout(timer);clearTimeout(notificationTimer);};
  });

  $effect(()=>{if(pendingImage&&(!image||efficiencyMode)){acceptImage();}});
  $effect(()=>{const dimensions=boardDimensions;const mounted=ready;if(mounted&&dimensions){const frame=requestAnimationFrame(initialView);return()=>cancelAnimationFrame(frame);}});
</script>

<svelte:head><title>Minesweeper Assistant</title><meta name="description" content="A local Minesweeper companion with screenshot recognition and Rust probability analysis." /></svelte:head>

<div class="app-shell">
  <header class="app-header">
    <div class="brand"><span class="brand-icon" aria-hidden="true">✳</span><div>Minesweeper<span>ASSISTANT</span></div><span class="version">BETA</span></div>
    <div class="header-actions"><span class="local-badge"><i></i> On-device · Rust engine</span><label class="mode-switch"><input type="checkbox" role="switch" bind:checked={efficiencyMode} /> Efficiency mode</label><button class="quiet" onclick={()=>showSettings=true}>Settings</button><button class="quiet" onclick={()=>showHelp=true}>How to use</button><button class="quiet" onclick={downloadSession}>Save session <span>↗</span></button></div>
  </header>
  <main>
    <aside class="sidebar left-sidebar">
      <div class="section-kicker">YOUR WORKSPACE</div>
      <h1>Read the board.<br /><span>Find your next move.</span></h1>
      <p class="subtle intro">Paste a screenshot, check the scan, and explore every move.</p>
      <section class="import-card" aria-label="Import screenshot">
        <div class="drop-area" role="region" aria-label="Drop a screenshot here" ondragover={e=>e.preventDefault()} ondrop={e=>{e.preventDefault();const file=e.dataTransfer?.files[0];if(file)void readImage(file);}}>
          <span class="upload-icon" aria-hidden="true">▧</span><strong>Bring your board</strong><p>Drop an image here or press <kbd>Ctrl</kbd> + <kbd>V</kbd></p>
          <div class="import-actions"><button class="primary" onclick={pasteImage}>Paste image</button><button class="secondary" onclick={()=>fileInput.click()}>Open image</button></div>
          <span class="tiny muted">PNG, JPEG or WebP · up to 20 MB</span>
        </div>
        <input class="hidden" bind:this={fileInput} type="file" accept="image/png,image/jpeg,image/webp" onchange={e=>{const file=e.currentTarget.files?.[0];if(file)void readImage(file);e.currentTarget.value='';}} />
      </section>

      <section class="capture-panel" aria-label="Screen capture">
        <div class="section-heading"><h2>Screen capture</h2><kbd>{bindingLabel(captureStatus.configuredHotkey)}</kbd></div>
        <button class="secondary full" disabled={scanning||capturing||choosingCapture} onclick={chooseCaptureArea}>{choosingCapture?'Selecting area…':captureStatus.region?'Change capture area':'Choose capture area'} <span>⌖</span></button>
        <button class="primary full" disabled={!captureStatus.region||scanning||capturing||choosingCapture} onclick={captureNow}>{capturing||scanning?'Capturing & scanning…':'Capture & analyze'} <span>↗</span></button>
        <button class="hotkey-toggle full" role="switch" aria-label="Global capture hotkey" aria-checked={captureStatus.hotkeyEnabled} class:enabled={captureStatus.hotkeyEnabled} disabled={hotkeySaving} onclick={()=>void toggleCaptureHotkey(!captureStatus.hotkeyEnabled)}><span>Hotkey {captureStatus.hotkeyEnabled?'on':'off'} · {bindingLabel(captureStatus.configuredHotkey)}</span><strong>{hotkeySaving?'…':captureStatus.hotkeyEnabled?'Pause':'Enable'}</strong></button>
        <p class="tiny muted">{captureStatus.hotkeyEnabled?'Turn off before typing in other apps. The key refreshes only your selected area.':'Paused: the key types normally in other apps. The capture button still works.'}</p>
        {#if captureStatus.region}<p class="tiny muted">{captureStatus.region.width} × {captureStatus.region.height} pixels · cursor excluded. Keep the game in the selected position; reselect if it moves.</p>{:else}<p class="tiny muted">Show your game, then select its board and mine counter. Button or global hotkey refreshes it without pasting.</p>{/if}
        {#if captureStatus.shortcutError}<p class="tiny capture-warning">Hotkey unavailable. Choose another in Settings; the button still works.</p>{/if}
        <button class="text-button" onclick={()=>showSettings=true}>Change capture hotkey</button>
      </section>

      {#if image}
        <section class="source-panel">
          <div class="section-heading"><h2>Source screenshot</h2><button class="text-button" onclick={()=>showSource=!showSource}>{showSource?'Hide':'Show'}</button></div>
          {#if showSource}
            <div class="source-stage" role="region" aria-label="Drag over screenshot to crop the board" onpointerdown={cropDown} onpointermove={cropMove} onpointerup={cropUp}>
              <img bind:this={imageElement} src={image} alt="Original Minesweeper screenshot. Drag to select a board crop." draggable="false" onload={e=>{imageWidth=(e.currentTarget as HTMLImageElement).naturalWidth;imageHeight=(e.currentTarget as HTMLImageElement).naturalHeight;}} />
              {#if cropStyle}<div class="crop-selection" style:left={`${cropStyle.x*100}%`} style:top={`${cropStyle.y*100}%`} style:width={`${cropStyle.width*100}%`} style:height={`${cropStyle.height*100}%`}></div>{/if}
            </div>
            <p class="tiny muted">Drag around the board to crop. {#if crop}<button class="text-button" onclick={()=>{crop=null;cropStart=null;cropEnd=null;}}>Reset crop</button>{/if}</p>
          {/if}
          <button class="primary full" aria-label={scanning?'Scanning screenshot':'Scan screenshot'} disabled={scanning} onclick={scanScreenshot}>{scanning?'Checking grid and cells…':'Scan screenshot'} <span>→</span></button>
          <button class="text-button grid-toggle" onclick={()=>showGrid=!showGrid}>{showGrid?'−':'+'} Adjust grid</button>
          {#if scan?.comparison&&['same','progress'].includes(scan.comparison.kind)}<div class="board-comparison" aria-label="Board comparison"><strong>{scan.comparison.kind==='progress'?'Continuing board':'Matching board'}</strong><p class="tiny muted">{scan.comparison.newlyRevealed} more revealed · {scan.comparison.unchangedCovered} still covered</p><details><summary>What was compared?</summary><p>{scan.comparison.message}</p></details></div>{/if}
          {#if scan?.counter}
            <div class="counter-helper" aria-label="Detected remaining flags">
              <strong>Remaining flags: {scan.counter.value}</strong>
              <p class="tiny muted">Top-left counter. {scan.counter.flagCount} placed flags were detected{scan.counter.totalMines!==null?`, giving ${scan.counter.totalMines} total mines`:'. The resulting mine total is outside this board’s range'}.</p>
              {#if !reviewed}<label class="check"><input type="checkbox" bind:checked={useScannedCounter} /> Use this counter when accepting the scan</label>{/if}
              <p class="tiny muted">Check the reading against the screenshot. The total is calculated from your reviewed flags.</p>
            </div>
          {/if}
          {#if showGrid}<div class="grid-options"><label class="check"><input type="checkbox" bind:checked={useGrid} /> Set dimensions manually</label><div class="two-fields"><label>Rows<input type="number" min="1" max="100" bind:value={gridRows} disabled={!useGrid} /></label><label>Columns<input type="number" min="1" max="100" bind:value={gridCols} disabled={!useGrid} /></label></div><p class="tiny muted">Crop tightly when setting dimensions. Otherwise rows and columns are detected automatically.</p></div>{/if}
        </section>
      {/if}

      <section class="settings-section">
        <div class="section-heading"><h2>Board settings</h2><span class="small-badge">{board.cols} × {board.rows}</span></div>
        <label class="field-label" for="total-mines">Total mines <span class="muted">on the entire board</span></label>
        <div class="mine-input"><span aria-hidden="true">✹</span><input id="total-mines" type="number" min="0" max={board.cells.length} value={board.totalMines??''} placeholder="Unknown" onchange={e=>{const raw=e.currentTarget.value;if(raw==='')changeMines(null);else{const n=Number(raw);if(Number.isInteger(n)&&n>=0&&n<=board.cells.length)changeMines(n);else error='Enter a valid total mine count.';}}} /></div>
        <button class="text-button" onclick={()=>showCounter=!showCounter}>{showCounter?'Hide counter helper':'Have a game counter instead?'}</button>
        {#if showCounter}<div class="counter-helper"><label>Displayed game counter<input type="number" bind:value={counterValue} /></label><p class="tiny muted">For counters that subtract every placed flag: total = counter + {flags} flags on this board.</p><button class="secondary full" disabled={counterValue===undefined} onclick={()=>{if(counterValue!==undefined&&Number.isInteger(counterValue))void useCounter(counterValue);else error='Enter a whole number for the counter.';}}>Use this total</button></div>{/if}
        <p class="check flag-trust">⚑ Flags are confirmed mines</p>
        <p class="tiny muted">Flags are confirmed mines.</p>
      </section>
      <div class="sidebar-links"><button class="secondary full" onclick={()=>showNew=true}>＋ New custom board</button><button class="text-button" onclick={()=>sessionInput.click()}>Open saved session</button><button class="text-button" onclick={demo}>Load example</button><input class="hidden" bind:this={sessionInput} type="file" accept=".json,application/json" onchange={e=>{const file=e.currentTarget.files?.[0];if(file)void openSession(file);e.currentTarget.value='';}} /></div>
      <p class="privacy-note"><span aria-hidden="true">◉</span> Your screenshots stay on this computer.</p>
    </aside>

    <section class="board-workspace" aria-label="Board workspace">
      <div class="board-toolbar"><div class="board-heading"><h2 title={name}>{name}</h2><div class="board-stats"><span><strong>{board.cols} × {board.rows}</strong></span><span class="detail-stat">{progress} revealed · {flags} flags</span><span class:good={analysis?.status==='exact'} class="status-badge"><i class:working={busy||scanning}></i>{capturing?'Capturing':scanning?'Scanning':!reviewed?'Scan review':busy?'Calculating':analysis?.status==='exact'?'Exact analysis':analysis?.status==='invalid'?'Check board':analysis?.status==='limited'?'Search limit':'Logic mode'}</span></div></div><div class="toolbar-actions"><button class="icon-button" title={`Undo${bindings.undo.length?` (${bindings.undo.map(bindingLabel).join(', ')})`:''}`} aria-label="Undo" disabled={!history.length} onclick={undo}>↶</button><button class="icon-button" title={`Redo${bindings.redo.length?` (${bindings.redo.map(bindingLabel).join(', ')})`:''}`} aria-label="Redo" disabled={!future.length} onclick={redo}>↷</button><span class="divider"></span><button class="icon-button target-button" aria-label="Show recommended square" title={best===null?"No recommendation yet":`Show blue target: ${coord(best)}`} disabled={best===null} onclick={showBest}>⌖</button><button class="text-button" style="margin-right:6px" onclick={fitBoard}>Fit</button><button class="icon-button" aria-label="Zoom out" disabled={zoom<=0.25} onclick={()=>void zoomAt(zoom/1.15)}>−</button><span class="zoom-label">{Math.round(zoom*100)}%</span><button class="icon-button" aria-label="Zoom in" disabled={zoom>=3} onclick={()=>void zoomAt(zoom*1.15)}>＋</button></div></div>
      <div class="board-scroll" class:panning use:navigateBoard>
        <div class="board-wrap" style:--cell={`${Math.round(43*zoom)}px`} style:--board-font-scale={boardFontScale/100}>
          <div class="column-coordinates" style:grid-template-columns={`repeat(${board.cols},var(--cell))`}>{#each Array(board.cols) as _,c}<span>{c+1}</span>{/each}</div>
          <div class="board-row"><div class="row-coordinates" style:grid-template-rows={`repeat(${board.rows},var(--cell))`}>{#each Array(board.rows) as _,r}<span>{r+1}</span>{/each}</div>
            <div class="board-grid" role="group" aria-label="Editable Minesweeper board" style:grid-template-columns={`repeat(${board.cols},var(--cell))`}>
              {#each board.cells as value,i}
                {@const odds=validAnalysis?.odds[i]}
                {@const step=stepForCell(i)}
                <button id={`cell-${i}`} class="cell" class:best-move={!efficiencyMode&&best===i&&value===-1&&!!validAnalysis} data-recommended={!efficiencyMode&&best===i&&value===-1&&!!validAnalysis} class:covered={value<0} class:revealed={value>=0} class:selected={selected===i} class:safe={!efficiencyMode&&overlay&&value<0&&(odds?.display?.highlight==='safe'||odds?.display?.highlight==='best')} class:mine={!efficiencyMode&&overlay&&value<0&&(odds?.display?.highlight==='mine'||odds?.display?.highlight==='worst')} class:risky={!efficiencyMode&&overlay&&odds?.kind==='risk'&&value<0} class:unreadable={value===-5} class:uncertain={!reviewed&&isUncertain(i)} class:conflict={analysis?.conflicts.includes(i)} style:--odds-color={odds?.display?.color??'#aab7c8'} class:plan-flag={step?.kind==='flag'} class:plan-chord={step?.kind==='chord'} class:plan-reveal={step?.kind==='reveal'&&plan?.status!=='guess'} class:plan-guess={step?.kind==='reveal'&&plan?.status==='guess'} class:plan-result={efficiencyMode&&planTargets.has(i)&&!step} class:plan-current={step?.number===activeStep+1} data-plan={step?.kind} data-step={step?.number} data-highlight={!efficiencyMode&&overlay?odds?.display?.highlight:undefined} data-muted={odds?.display?.muted??false} title={riskTitle(i)} data-clue={value} aria-label={describe(i)} aria-pressed={selected===i} onclick={()=>selectCell(i)} oncontextmenu={e=>{e.preventDefault();selected=i;editCell(value===-2?-1:-2);}}>{#if step}<span class="action-badge">{step.number}</span>{/if}
                  {#if value>0}<span class="clue" style:color={clueColor(value)}>{value}</span>{:else if value===-2}<span class="cell-symbol flag">⚑</span>{:else if value===-3}<span class="cell-symbol">✹</span>{:else if value===-4}<span class="cell-symbol">✓</span>{:else if value===-5}<span class="cell-symbol">?</span>{:else if efficiencyMode&&step?.kind==='flag'}<span class="cell-symbol planned-symbol">⚑</span>{:else if efficiencyMode&&step?.kind==='reveal'}<span class="cell-symbol planned-symbol">↗</span>{:else if value===-1&&!efficiencyMode&&overlay}<span class="cell-percent" use:fitCellText><span class="percent-value">{odds?.probability!==null&&odds?.probability!==undefined?Math.round((showSafe?1-odds.probability:odds.probability)*100):''}</span><span class="percent-sign">{odds?.probability!==null&&odds?.probability!==undefined?'%':''}</span></span>{#if odds?.kind==='safe'||odds?.kind==='mine'}<span class="certainty-mark" aria-label={odds.kind==='safe'?'Proven safe':'Guaranteed mine'}>{odds.kind==='safe'?'✓':'⚑'}</span>{/if}{/if}
                </button>
              {/each}
            </div>
          </div>
        </div>
      </div>
      <div class="board-bottom">{#if efficiencyMode}<div class="legend"><span><i style="background:#e7b061"></i> Flag</span><span><i style="background:#7eb8ff"></i> Chord</span><span><i class="safe-dot"></i> Reveal</span></div><span class="tiny muted">Numbered actions · no percentages</span>{:else}<div class="legend"><span title="Blue marks one recommended move. Other cells may have equal odds; blue does not guarantee safety."><i class="best-dot"></i> Blue target</span><span><i class="safe-dot"></i> Lowest risk</span><span><i class="risk-dot"></i> Common odds</span><span><i class="mine-dot"></i> Highest risk</span></div><label class="check"><input type="checkbox" bind:checked={overlay} /> Show odds</label><select aria-label="Probability display" bind:value={showSafe}><option value={false}>Mine risk</option><option value={true}>Safe chance</option></select>{/if}</div>
      <footer class="analysis-footer"><span>{busy?'Working in Rust…':analysis?`${analysis.elapsedMs} ms · ${analysis.components} frontier groups · ${analysis.nodes.toLocaleString()} search nodes${analysis.cachedComponents?` · ${analysis.cachedComponents} cached`:""}${analysis.pooledCells?` · ${analysis.pooledCells} pooled`:""}`:'Ready when you are.'}</span><span class="navigation-help" title="Wheel to zoom · Ctrl + drag or middle-drag to pan">Wheel: zoom · Ctrl + drag: pan</span></footer>
    </section>

    <aside class="sidebar right-sidebar">
      <section class="edit-dock" aria-label="Cell editing tools"><div class="edit-dock-title"><strong>{selected===null?'Select a square to edit':`${coord(selected)} · ${board.cells[selected]>=0?`Revealed ${board.cells[selected]}`:labels[board.cells[selected]]}`}</strong><span>{activeTool!==null?'Paint mode · Esc to stop':'Enter the result you see in the real game'}</span></div><div class="edit-tools">{#each tools as tool}<button title={toolTitle(tool)} aria-label={toolTitle(tool)} style:grid-column={tool.value>=0?tool.value%3+1:4+([-1,-2,-4,-5].indexOf(tool.value)%2)} style:grid-row={tool.value>=0?Math.floor(tool.value/3)+1:Math.floor([-1,-2,-4,-5].indexOf(tool.value)/2)+1} class:active={activeTool===tool.value} onclick={()=>{if(activeTool!==null)activeTool=tool.value;if(selected!==null)editCell(tool.value);else activeTool=tool.value;}}>{tool.label}</button>{/each}<label class="paint-toggle"><input type="checkbox" checked={activeTool!==null} onchange={e=>{activeTool=e.currentTarget.checked?0:null;}} /> Paint</label></div><p class="tiny muted">Shortcuts are customizable in Settings · Right-click: flag · Esc: stop painting</p></section>
      {#if !reviewed}<div class="banner review"><span>◈</span><div><strong>{scan?'Check the scan before solving':'Screenshot ready to scan'}</strong><p>{scan?`${unresolved} unreadable cells; ${needsReview} ambiguous symbols. ${scan?.reviewReasons?.join(' ')??'Blank cells need no individual review.'}`:'Choose Scan screenshot. Your previous board stays here until a scan completes.'}</p></div>{#if scan}<button class="secondary" onclick={nextUncertain}>Next to review</button><button class="primary" disabled={unresolved>0} onclick={acceptReview}>Accept scan</button>{/if}</div>{/if}

      {#if efficiencyMode}
        <div class="section-kicker">FEWER CLICKS</div>
        <h2 class="right-title">Your next sequence.</h2>
        <p class="subtle">Flags count as clicks. Place only the ones that help the plan.</p>
        {#if plan?.actions.length}
          <section class="sequence-summary" class:guess={plan.status==='guess'}>
            <strong>{plan.status==='guess'?'Guess required':`${plan.clicks} click${plan.clicks===1?'':'s'} · ${plan.targets.length} safe square${plan.targets.length===1?'':'s'}`}</strong>
            <p>{plan.status==='guess'?'No provably safe action is available. This is a lowest-risk fallback; click efficiency after the reveal is unknown.':`This sequence opens at least ${plan.targets.length} known-safe squares. Extra openings are possible.`}</p>
          </section>
          <ol class="sequence-list" aria-label="Efficiency sequence">
            {#each plan.actions as action,index}
              <li><button class="sequence-action" class:current={activeStep===index} class:completed={activeStep>index} data-action={action.kind} onclick={()=>showStep(index)}><span class="step-number">{index+1}</span><span><strong>{action.kind==='flag'?'Flag':action.kind==='chord'?'Chord':'Reveal'} {coord(action.cell)}</strong><small>{action.kind==='flag'?'Right-click · proven mine':action.kind==='chord'?'Click the revealed number':plan.status==='guess'?'Guess · not guaranteed':'Left-click · proven safe'}</small></span></button><p class="tiny muted">{action.explanation}</p></li>
            {/each}
          </ol>
          {#if activeStep<plan.actions.length}<button class="secondary full" onclick={()=>activeStep+=1}>Mark step {activeStep+1} done <span>✓</span></button>{:else}<div class="awaiting-screenshot" role="status"><strong>Ready for the next screenshot</strong><p>Paste it with Ctrl+V. The app will scan and replan.</p></div>{/if}
          <button class="primary full next-screenshot" onclick={pasteImage}>Paste next screenshot <span>▧</span></button>
          <p class="tiny muted">Perform these actions in the actual game. Marking a step done here does not change the board or click the game.</p>
          {#if plan.status==='safe'}<section class="efficiency-details"><h2>Why this sequence?</h2><p class="tiny muted">{plan.message}</p><span class="small-badge">{plan.optimal?'Minimum for current known work':'Best found within the planning budget'}</span></section>{/if}
        {:else}<section class="empty-recommendation"><strong>{!reviewed?'Review the scan':busy?'Finding a click-saving sequence…':plan?.status==='done'?'No more safe squares to open':'No sequence available yet'}</strong><p>{plan?.message??'Import a board or enter clues. Unreadable and inconsistent scans need review.'}</p></section>{/if}
        <section class="method-note"><h2>Flag, chord, or reveal?</h2><p>One flag costs one click. A chord costs one action and can clear several neighbors; flags can be reused by nearby chords. Direct reveals win when flagging costs more. Proven zero openings are included.</p><p class="tiny">The plan uses current information. After its first reveal or chord, paste a new screenshot—new clues or openings may change the most efficient continuation.</p><p class="tiny">On minesweeper.online, chord by clicking a revealed number with the required adjacent flags, or use your configured chord gesture.</p></section>
      {:else}
      <div class="section-kicker">THE NEXT MOVE</div>
      <h2 class="right-title">A little more certainty.</h2>
      <div class="summary-counts"><div><span class="safe-text">{validAnalysis?safeCount:'—'}</span><small>safe squares</small></div><div><span class="mine-text">{validAnalysis?mineCount:'—'}</span><small>proven mines</small></div></div>
      {#if best!==null && validAnalysis}
        {@const bestOdds=validAnalysis.odds[best]}
        <section class="recommendation" class:guess={bestOdds.kind!=='safe'}><div class="recommendation-label">{bestOdds.kind==='safe'?'SAFE TO CLEAR':'LOWEST MINE RISK'}</div><div class="recommendation-coordinate">{coord(best)}<span>{bestOdds.kind==='safe'?'✓':'↗'}</span></div><p>{bestOdds.kind==='safe'?'No mine in any compatible arrangement.':`${percent(bestOdds.probability,bestOdds.kind,false)} mine risk. This move is a guess.`}</p><button class="secondary full" onclick={()=>{selected=best;document.getElementById(`cell-${best}`)?.scrollIntoView({block:'nearest',inline:'nearest'});}}>Show blue target <span>⌖</span></button></section>
      {:else}<section class="empty-recommendation"><span aria-hidden="true">◈</span><strong>{!reviewed?'Review your scan':busy?'Reading the possibilities…':analysis?.status==='invalid'?'Check the board':analysis?.status==='exact'?'No move to recommend':'Your next move appears here'}</strong><p>{!reviewed?'Correct uncertain cells and accept the scan.':analysis?.status==='limited'?'The exact search reached its budget. Only proven deductions are shown.':'Enter clues and a mine total, or import a screenshot to begin.'}</p></section>{/if}
      {#if validAnalysis&&validAnalysis.recommendations.length>1}<section class="move-list"><div class="section-heading"><h2>Other options</h2><span class="muted tiny">mine risk</span></div>{#each validAnalysis.recommendations.slice(1,6) as i}<button class="move-option" onclick={()=>selected=i}><span>{coord(i)}</span><strong style:color={validAnalysis.odds[i].display?.color} class:safe-text={validAnalysis.odds[i].kind==='safe'}>{validAnalysis.odds[i].kind==='safe'?'Safe':percent(validAnalysis.odds[i].probability,validAnalysis.odds[i].kind,false)}</strong></button>{/each}</section>{/if}
      <section class="inspector"><div class="section-heading"><h2>Square inspector</h2>{#if selected!==null}<span class="small-badge">{coord(selected)}</span>{/if}</div>{#if selected!==null}<strong class="inspector-value">{board.cells[selected]>=0?`Revealed ${board.cells[selected]}`:labels[board.cells[selected]]}</strong>{#if selectedOdds?.probability!==null&&selectedOdds?.probability!==undefined}<div class="inspector-risk" style:color={selectedOdds.display?.color}>{percent(selectedOdds.probability,selectedOdds.kind)}<span>{showSafe?'safe chance':'mine risk'}</span></div><p class="tiny muted">{selectedOdds.reason}</p>{#if selectedOdds.numerator&&analysis?.solutions}<details><summary>Exact fraction</summary><p class="fraction">{selectedOdds.numerator} / {analysis.solutions}</p></details>{/if}{:else}<p class="subtle">{board.cells[selected]>=0?'This clue constrains the adjacent covered squares.':'No percentage available yet.'}</p>{/if}{#if scan}<div class="scan-detail"><strong>Screenshot interpretation</strong><p>{scan.notes[selected]}</p><span class="tiny muted">{scan.confidence[selected]>=1?'Manually reviewed':scan.confidence[selected]>=0.8?'Stronger visual match':'Review recommended'} · visual scores are not mine odds</span></div>{/if}{:else}<p class="subtle">Select any square to see its state, odds, and explanation.</p>{/if}</section>
      <section class="method-note"><span class="tiny section-kicker">HOW TO READ THE ODDS</span><p>{analysis?.message??'Percentages count compatible mine placements using your clues and total. A screenshot cannot reveal what is under a covered square.'}</p>{#if board.totalMines===null}<strong>Mine total unknown: logic only.</strong>{/if}<p class="tiny">Percentages are rounded. Only ✓ and ⚑ certainty markers prove safe cells or mines; a rounded 0% or 100% can still be a guess.</p></section>
      {/if}
    </aside>
  </main>
  {#if notification}<div class="toast" class:toast-error={notification.kind==='error'} role={notification.kind==='error'?'alert':'status'}><span class="toast-icon" aria-hidden="true">{notification.kind==='error'?'!':'✓'}</span><p>{notification.message}</p><button aria-label="Dismiss notification" onclick={dismissNotification}>×</button></div>{/if}
</div>

{#if pendingImage}<div class="modal-backdrop"><div class="modal" use:focusModal tabindex="-1" role="dialog" aria-modal="true" aria-labelledby="replace-title"><h2 id="replace-title">Use this screenshot?</h2><p>Your current position will stay in Undo. Scan results replace the board after recognition; review them before accepting.</p><div class="modal-actions"><button class="secondary" onclick={()=>pendingImage=null}>Cancel</button><button class="primary" onclick={acceptImage}>Use screenshot</button></div></div></div>{/if}
{#if showNew}<div class="modal-backdrop"><div class="modal" use:focusModal tabindex="-1" role="dialog" aria-modal="true" aria-labelledby="new-title"><h2 id="new-title">Create a custom board</h2><p>Start covered, then enter the clues you can see.</p><div class="two-fields"><label>Rows<input type="number" min="1" max="100" bind:value={newRows}/></label><label>Columns<input type="number" min="1" max="100" bind:value={newCols}/></label></div><label>Total mines<input type="number" min="0" bind:value={newMines} placeholder="Unknown"/></label><p class="tiny muted">Up to 2,500 cells. Large connected frontiers may exceed the exact search budget.</p><div class="modal-actions"><button class="secondary" onclick={()=>showNew=false}>Cancel</button><button class="primary" onclick={createBoard}>Create board</button></div></div></div>{/if}
{#if showHelp}<div class="modal-backdrop"><div class="modal help-modal" use:focusModal tabindex="-1" role="dialog" aria-modal="true" aria-labelledby="help-title"><span class="section-kicker">A QUICK START</span><h2 id="help-title">From screenshot to next move.</h2><ol><li><strong>Import your board.</strong> Paste with Ctrl+V or open an image. Drag on the source image if it needs cropping.</li><li><strong>Scan and check.</strong> Rows and columns are detected automatically. Use Adjust grid when needed. Clear scans are accepted automatically. Correct ambiguous cells and accept when review is needed.</li><li><strong>Set total mines.</strong> Enter the total for the entire board, or use the counter helper. Leave it blank for logical deductions only.</li><li><strong>Make your move in the game.</strong> Then select that square here and enter the revealed number. 0 means a blank revealed cell; S means safe with its clue not yet entered.</li><li><strong>Keep it current.</strong> Odds recalculate after edits. Efficiency mode shows numbered flags, chords, or reveals instead of odds. Follow them in the game, then paste a new screenshot to scan and replan automatically.</li></ol><p class="subtle">The scanner compares several grid alignments, learns covered and revealed backgrounds, and checks glyph shape and color in Rust. Unfamiliar fonts, borderless boards, flags, and unusual themes can require manual correction. Flags are always confirmed mines. Recognition scores are not calibrated confidence percentages.</p><button class="primary full" onclick={()=>showHelp=false}>Got it</button></div></div>{/if}

{#if showSettings}<Settings bind:bindings bind:fontScale={boardFontScale} bind:captureStatus {focusModal} onclose={()=>showSettings=false}/>{/if}

<style>
  .hotkey-toggle{display:flex;align-items:center;gap:8px;padding:9px 10px;margin-top:9px;border-radius:6px;border:1px solid #415269;background:#1c2837;color:#afc1d6;font-size:11px}.hotkey-toggle.enabled{background:#173b31;border-color:#3c9373;color:#b9f2d8}.hotkey-toggle strong{font-size:10px;color:#9cddff}
  .capture-panel{margin-top:20px;padding-top:18px;border-top:1px solid #2c3a48}.capture-panel .section-heading{margin-bottom:10px;flex-wrap:wrap}.capture-panel .section-heading kbd{font-size:9px;color:#acd7f4}.capture-panel .primary{margin-top:8px}.capture-warning{color:#f0ba82}
  .board-grid .cell.best-move,.board-grid .cell.best-move:hover{background:#0878ee;border-color:#9ce6ff;color:#fff;outline:2px solid #4fd6ff;outline-offset:1px;box-shadow:0 0 0 4px #17bbff45,0 0 20px #13b8ff99;z-index:2}.board-grid .cell.best-move .cell-percent{color:#fff;text-shadow:0 1px 2px #00316c}.board-grid .cell.best-move .certainty-mark{color:#fff}.board-grid .cell.best-move.selected{outline-color:#fff}.best-dot{background:#2ac9ff;box-shadow:0 0 6px #2ac9ff}.target-button{color:#8de4ff;border-color:#3982ac;background:#15354a}.target-button:hover{background:#1f4d6a}
  .mode-switch{display:flex;align-items:center;gap:7px;padding:8px 10px;background:#1b2c39;border:1px solid #3c576b;border-radius:7px;font-size:11px;color:#c3dcf0;white-space:nowrap}
  .sequence-summary{padding:15px;border:1px solid #376451;border-radius:9px;background:#19352d;margin:20px 0}.sequence-summary.guess{border-color:#85623f;background:#382b20}.sequence-summary strong{font-size:16px;color:#b1f0d0}.sequence-summary.guess strong{color:#f0c38a}.sequence-summary p{font-size:11px;color:#abc2b8;margin:8px 0 0}.sequence-list{list-style:none;margin:0;padding:0}.sequence-list>li{margin-bottom:17px}.sequence-list>li>p{margin:8px 3px}.sequence-action{width:100%;display:flex;gap:10px;align-items:center;text-align:left;background:#1b2938;border:1px solid #3a4b60;border-radius:8px;padding:11px 10px;color:#cedff0}.sequence-action.current{border-color:#7eb8ff;box-shadow:0 0 0 1px #7eb8ff33}.sequence-action.completed{opacity:.55}.sequence-action strong{font-size:12px;display:block}.sequence-action small{font-size:10px;display:block;margin-top:4px;color:#91aac3}.step-number{display:grid;place-items:center;border-radius:50%;width:25px;height:25px;background:#365375;color:#e6f2ff;font-weight:700;flex-shrink:0}.efficiency-details{margin-top:23px;padding-top:18px;border-top:1px solid #303d4b}.next-screenshot{margin-top:12px}.awaiting-screenshot{padding:13px;border:1px dashed #6f8a72;border-radius:8px;color:#acd6bb}.awaiting-screenshot strong{font-size:12px}.awaiting-screenshot p{font-size:11px;margin-bottom:0}
  .cell.plan-flag{background:#44311d;border-color:#d59c50;color:#ffd595}.cell.plan-chord{background:#1e3854;border-color:#69adff}.cell.plan-reveal{background:#153d31;border-color:#56d5aa;color:#8df2c1}.cell.plan-guess{background:#493823;border-color:#e4b46d;color:#ffd58f}.cell.plan-result{border-color:#3e7460;box-shadow:inset 0 0 0 1px #3e746066}.cell.plan-flag:hover{background:#44311d}.cell.plan-chord:hover{background:#1e3854}.cell.plan-reveal:hover{background:#153d31}.cell.plan-guess:hover{background:#493823}.cell.plan-current{outline:2px solid #d5edff;outline-offset:2px;z-index:2}.action-badge{position:absolute;left:-2px;top:-4px;min-width:14px;height:14px;padding:1px 3px;background:#90c6ff;color:#112439;border:1px solid #d8ecff;border-radius:4px;font-size:9px;font-weight:800;line-height:11px;z-index:3}.planned-symbol{opacity:.85}
  :global(:root){color-scheme:dark}
  .board-scroll.panning,.board-scroll.panning :global(*){cursor:grabbing!important;user-select:none}
  :global(*){box-sizing:border-box} :global(body){margin:0;background:#0e1319;color:#e7edf4;font-family:'Segoe UI',sans-serif;font-size:13px;-webkit-font-smoothing:antialiased} :global(button),:global(input),:global(select){font:inherit} :global(button){cursor:pointer} :global(button:disabled){cursor:default;opacity:.4} :global(button:focus-visible),:global(input:focus-visible),:global(select:focus-visible),:global(summary:focus-visible){outline:2px solid #7eb5ff;outline-offset:3px} :global(button){transition:background .15s,border-color .15s} :global(input[type=number]){width:100%;min-width:0;border:1px solid #35404e;border-radius:7px;padding:9px 10px;background:#10161e;color:#e7edf4} :global(input[type=checkbox]){accent-color:#efad70;width:14px;height:14px} :global(select){background:#161e28;border:1px solid #34404e;color:#b7c4d4;border-radius:6px;padding:5px} :global(h2){font-size:13px;font-weight:600;margin:0} :global(p){line-height:1.6} .hidden{display:none}.app-header{height:76px;display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #26303c;padding:0 28px;background:#111820}.brand{display:flex;align-items:center;gap:12px;font-size:17px;font-weight:650;letter-spacing:-.4px}.brand div>span{display:block;font-size:9px;font-weight:600;letter-spacing:2.5px;margin-top:2px;color:#8f9dae}.brand-icon{display:grid;place-items:center;width:36px;height:36px;border:1px solid #725140;border-radius:11px;background:#382b24;color:#f2b47e;font-size:29px}.version{font-size:8px;color:#9cabbc;border:1px solid #354151;border-radius:4px;padding:3px 5px;letter-spacing:1px;margin-left:4px}.header-actions{display:flex;gap:13px;align-items:center}.local-badge{font-size:11px;color:#8ea69c;margin-right:16px;display:flex;gap:7px;align-items:center}.local-badge i{width:5px;height:5px;border-radius:50%;background:#68c6a5}.quiet{background:transparent;color:#b7c3d0;border:1px solid #303b48;border-radius:7px;padding:8px 12px}.quiet span{margin-left:10px} main{display:grid;grid-template-columns:260px minmax(390px,1fr) 264px;height:calc(100vh - 76px);min-height:0;overflow:hidden}.sidebar{padding:27px 20px;background:#111820;overflow-y:auto}.left-sidebar{border-right:1px solid #26303c}.right-sidebar{border-left:1px solid #26303c}.section-kicker{font-size:9px;font-weight:650;letter-spacing:1.5px;color:#8292a6}.left-sidebar h1{font-size:21px;letter-spacing:-.65px;line-height:1.35;margin:13px 0 10px;font-weight:600}.left-sidebar h1 span{color:#a4b2c2}.subtle{color:#93a3b6;font-size:12px}.intro{margin-bottom:23px}.import-card{border:1px dashed #394554;border-radius:10px;background:#161e28}.drop-area{padding:18px 9px;text-align:center;display:flex;flex-direction:column;align-items:center}.upload-icon{display:grid;place-items:center;width:33px;height:33px;border-radius:9px;background:#222d39;color:#a9bed3;font-size:23px;margin-bottom:11px}.drop-area strong{font-size:12px;font-weight:600}.drop-area p{font-size:10px;color:#91a0b2;margin:8px 0 16px}kbd{font:inherit;font-size:9px;border:1px solid #354252;padding:1px 3px;border-radius:3px}.import-actions{display:flex;gap:7px;margin-bottom:13px}.primary,.secondary{display:inline-flex;align-items:center;justify-content:center;gap:8px;min-height:33px;padding:8px 11px;border-radius:6px;font-size:11px;font-weight:650;border:1px solid transparent}.primary{background:#edb17d;color:#271e17;border-color:#edb17d}.primary:hover{background:#f7c291}.secondary{background:#202a36;border-color:#384555;color:#d5dfeb}.secondary:hover{background:#2a3746}.full{width:100%;justify-content:space-between}.tiny{font-size:10px;line-height:1.6}.muted{color:#8192a6}.text-button{background:transparent;color:#a1b8d0;border:0;padding:0;font-size:10px;text-align:left}.text-button:hover{color:#e1edfb}.settings-section{margin-top:27px;padding-top:22px;border-top:1px solid #28323f}.section-heading{display:flex;justify-content:space-between;align-items:center;margin-bottom:15px;gap:6px}.small-badge{font-size:10px;font-variant-numeric:tabular-nums;padding:3px 6px;border:1px solid #34404e;border-radius:4px;color:#a5b4c7}.field-label{font-size:11px;display:block;margin-bottom:8px}.field-label>span{display:block;margin-top:3px;font-size:10px}.mine-input{position:relative;margin-bottom:8px}.mine-input>span{position:absolute;top:8px;left:12px;font-size:19px;color:#9bafc5}.mine-input input{padding-left:39px;font-size:18px}.check{display:flex;gap:6px;align-items:center;font-size:10px;color:#b5c2d0;line-height:1.5}.flag-trust{margin-top:20px}.settings-section>p{margin:6px 0 0}.sidebar-links{display:flex;flex-direction:column;gap:14px;margin-top:25px;padding-top:22px;border-top:1px solid #28323f}.privacy-note{margin-top:26px;font-size:9px;color:#75869a;display:flex;gap:6px}.source-panel{margin-top:23px}.source-stage{position:relative;border:1px solid #394658;border-radius:6px;overflow:hidden;touch-action:none;user-select:none;cursor:crosshair;line-height:0}.source-stage img{width:100%;height:auto;display:block}.crop-selection{position:absolute;border:2px solid #edb17d;background:#edb17d20;pointer-events:none;box-shadow:0 0 0 2000px #0003}.grid-toggle{margin-top:11px}.grid-options,.counter-helper{padding:12px;background:#18212c;border:1px solid #303d4c;border-radius:7px;margin-top:10px}.two-fields{display:grid;grid-template-columns:1fr 1fr;gap:10px}.two-fields label,.counter-helper>label,.modal>label{display:block;font-size:11px;color:#a8b8ca}.two-fields input,.counter-helper input,.modal>label input{margin-top:7px}.grid-options .two-fields{margin-top:12px}.board-workspace{padding:28px 26px 16px;min-width:0;min-height:0;display:flex;flex-direction:column}.status-badge{display:flex;align-items:center;gap:6px;background:#222a35;border:1px solid #374353;border-radius:20px;padding:6px 10px;font-size:10px;color:#acbace;white-space:nowrap}.status-badge i{width:5px;height:5px;background:#b9a179;border-radius:50%}.status-badge.good{background:#152e2b;border-color:#254d43;color:#83d8ba}.status-badge.good i{background:#66cfaa}.working{animation:pulse 1s infinite}.board-toolbar{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:10px 0 14px}.board-stats{display:flex;gap:15px;font-size:10px;color:#7f90a5}.board-stats strong{color:#c4d0de;font-weight:500}.toolbar-actions{display:flex;align-items:center;gap:3px}.icon-button{border:1px solid #303c4b;border-radius:5px;background:#18212c;color:#a1b5cb;width:27px;height:27px;font-size:16px}.icon-button:hover{background:#253244}.zoom-label{font-size:10px;color:#7c8da2;min-width:35px;text-align:center}.divider{height:16px;width:1px;background:#344050;margin:0 6px}.board-scroll{min-height:140px;display:flex;align-items:center;justify-content:flex-start;overflow:auto;padding:23px 18px 27px;border:1px solid #2b3746;border-radius:11px 11px 0 0;background:radial-gradient(ellipse at center,#1a2430 0%,#141c26 80%);flex:1;max-height:none}.board-wrap{margin:auto;flex-shrink:0}.column-coordinates{display:grid;gap:3px;padding-left:25px;margin-bottom:9px}.column-coordinates span,.row-coordinates span{font-size:10px;text-align:center;color:#64778f;font-variant-numeric:tabular-nums}.board-row{display:flex;gap:9px}.row-coordinates{display:grid;gap:3px;width:16px;flex-shrink:0}.row-coordinates span{align-self:center}.board-grid{display:grid;gap:3px}.cell{width:var(--cell);height:var(--cell);padding:0;border:1px solid #42526a;border-bottom-width:3px;border-radius:5px;display:flex;align-items:center;justify-content:center;background:#29394d;color:#afc1d8;position:relative;line-height:1;font-variant-numeric:tabular-nums}.cell.covered:hover{background:#3a4d65;border-color:#748ba8}.cell.revealed{background:#17212d;border-color:#253244;border-bottom-width:1px}.cell .clue{font-size:calc(var(--cell)*.43*var(--board-font-scale,1));font-weight:700}.cell[data-clue='1'] .clue{color:#83b6f4}.cell[data-clue='2'] .clue{color:#77cba3}.cell[data-clue='3'] .clue{color:#e68f94}.cell[data-clue='4'] .clue{color:#b2a0e7}.cell[data-clue='5'] .clue{color:#d7a389}.cell[data-clue='6'] .clue{color:#7bc8d8}.cell[data-clue='7'] .clue{color:#b9c9dd}.cell[data-clue='8'] .clue{color:#a6b2c3}.cell.safe{background:#103d31;border-color:#25845c;color:#36f9a3}.cell.mine{background:#4a1c29;border-color:#a63d51;color:#ff3038}.cell.risky{color:var(--odds-color)}.cell.safe:hover{background:#164e3d}.cell.mine:hover{background:#5c2533}.cell.unreadable{background:#504129;color:#f2cd8d;border-color:#9c7f47}.cell.uncertain{outline:1px dashed #bd9d61;outline-offset:-4px}.cell.selected{outline:2px solid #edb17d;outline-offset:2px;z-index:1}.cell.conflict{box-shadow:inset 0 0 0 2px #ff899a}.cell-symbol{font-size:calc(var(--cell)*.42*var(--board-font-scale,1))}.cell-symbol.flag{color:#e6bc82}.cell-percent{display:flex;align-items:baseline;color:var(--odds-color);font-size:calc(var(--cell)*.43*var(--board-font-scale,1));letter-spacing:-.5px;font-weight:750;line-height:1}.percent-sign{font-size:.62em;margin-left:1px}.certainty-mark{position:absolute;right:2px;top:2px;font-size:calc(var(--cell)*.19*var(--board-font-scale,1));line-height:1;color:var(--odds-color)}.cell.mine .cell-symbol{color:#ff3038}.board-bottom{display:flex;gap:15px;align-items:center;justify-content:space-between;background:#151e28;border:1px solid #2b3746;border-top:0;border-radius:0 0 10px 10px;padding:12px 15px}.legend{display:flex;gap:11px;font-size:9px;color:#8fa0b6}.legend span{display:flex;align-items:center;gap:5px}.legend i{width:6px;height:6px;border-radius:2px}.safe-dot{background:#36f9a3}.risk-dot{background:#8b96a7}.mine-dot{background:#ff3038}.board-bottom select{font-size:10px}.edit-dock{flex-shrink:0;margin-top:15px;padding:15px;background:#151d27;border:1px solid #2d3848;border-radius:9px}.edit-dock-title{display:flex;flex-direction:column;gap:5px}.edit-dock-title strong{font-size:11px;font-weight:500;color:#cbd7e5}.edit-dock-title>span{font-size:10px;color:#7d90a7}.edit-tools{display:flex;gap:4px;flex-wrap:wrap;align-items:center;margin-top:12px}.edit-tools>button{display:grid;place-items:center;width:29px;height:30px;border:1px solid #3a485c;border-radius:5px;color:#bacee6;background:#223044;font-size:13px}.edit-tools>button.active{border-color:#edb17d;color:#edb17d;background:#45382d}.paint-toggle{display:flex;align-items:center;font-size:9px;color:#8c9fb8;margin-left:4px;gap:3px}.edit-dock p{margin:10px 0 0;font-size:9px}.analysis-footer{display:flex;justify-content:space-between;font-size:9px;color:#63778e;padding:14px 2px 0}.right-title{font-size:17px;letter-spacing:-.4px;margin-top:13px}.summary-counts{display:grid;grid-template-columns:1fr 1fr;padding:22px 0;margin-bottom:3px;gap:8px}.summary-counts>div{display:flex;flex-direction:column;gap:3px}.summary-counts>div+div{border-left:1px solid #303b49;padding-left:20px}.summary-counts span{font-size:28px;font-weight:600;font-variant-numeric:tabular-nums}.summary-counts small{color:#8ea0b7;font-size:10px}.safe-text{color:#80d5b2}.mine-text{color:#de9aaa}.recommendation{border:1px solid #346156;border-radius:9px;background:linear-gradient(135deg,#18352f,#16272a);padding:17px 14px}.recommendation.guess{border-color:#6a5843;background:linear-gradient(135deg,#362b22,#20252b)}.recommendation-label{color:#86d5b4;font-size:9px;letter-spacing:1.2px;font-weight:700}.guess .recommendation-label{color:#e5ba83}.recommendation-coordinate{font-size:26px;font-weight:600;letter-spacing:-1px;display:flex;justify-content:space-between;margin-top:12px;color:#d6eee3}.recommendation-coordinate>span{font-size:20px;align-self:center}.recommendation p{font-size:11px;color:#9ebcb0;margin:9px 0 16px}.guess p{color:#bcac99}.recommendation .secondary{background:#24443d;border-color:#3b6156;color:#c7e6d8}.guess .secondary{background:#44392c;border-color:#6a5944;color:#dfc8a9}.empty-recommendation{border:1px dashed #364555;border-radius:9px;padding:22px 15px;text-align:center}.empty-recommendation>span{display:block;font-size:25px;color:#708aa8;margin-bottom:13px}.empty-recommendation strong{font-size:12px;font-weight:500}.empty-recommendation p{font-size:11px;color:#869bb4}.move-list{margin-top:25px}.move-list .section-heading{margin-bottom:9px}.move-option{display:flex;justify-content:space-between;width:100%;background:transparent;border:0;border-bottom:1px solid #253140;padding:10px 2px;color:#a4b5ca;font-size:11px}.move-option:hover{background:#1b2837}.move-option strong{font-weight:500}.inspector{margin-top:27px;padding-top:23px;border-top:1px solid #2c3847}.inspector-value{font-size:12px;font-weight:500;color:#b7c8dd}.inspector-risk{font-size:27px;letter-spacing:-1px;font-weight:600;color:#e4c29c;margin-top:13px}.inspector-risk>span{font-size:10px;letter-spacing:0;font-weight:400;color:#8ca0b7;margin-left:7px}details{font-size:10px;color:#9aafc8;margin-top:15px}summary{cursor:pointer}.fraction{overflow-wrap:anywhere;font-family:monospace;font-size:10px;color:#b6cbe2}.scan-detail{margin-top:15px;padding:12px;background:#1b2532;border-radius:7px}.scan-detail strong{font-size:10px;color:#c1ccd9}.scan-detail p{font-size:10px;color:#91a5be}.method-note{margin-top:28px;padding-top:21px;border-top:1px solid #2c3847}.method-note>p{font-size:10px;color:#8398b0}.method-note>strong{font-size:10px;color:#d6bb8e;font-weight:500}.banner{display:flex;gap:12px;align-items:center;border:1px solid #5f5141;background:#29251f;border-radius:8px;padding:13px;margin-bottom:14px;color:#d6bd99}.banner>span{font-size:19px}.banner p{font-size:11px;margin:0;flex:1}.banner strong{font-size:12px}.banner>button:not(.primary):not(.secondary){background:none;border:0;color:inherit}.banner.review{flex-wrap:wrap}.banner.review>div{flex:1;min-width:180px}.banner.review p{margin-top:3px;color:#9f9d95}.modal-backdrop{position:fixed;inset:0;background:#050a12c9;backdrop-filter:blur(5px);display:flex;align-items:center;justify-content:center;z-index:10;padding:30px}.modal{width:420px;max-width:100%;border:1px solid #3c4b5e;border-radius:14px;background:#17212d;padding:27px;box-shadow:0 20px 80px #0006;max-height:90vh;overflow:auto}.modal h2{font-size:21px;letter-spacing:-.5px;margin:0 0 13px}.modal p{font-size:12px;color:#a3b5cd}.modal-actions{display:flex;gap:10px;justify-content:flex-end;margin-top:24px}.modal>.two-fields{margin-bottom:17px}.help-modal{width:580px}.help-modal .section-kicker{display:block;margin-bottom:11px}.help-modal ol{padding-left:19px;color:#a3b5cd;font-size:12px;line-height:1.7}.help-modal li{padding-left:5px;margin-bottom:14px}.help-modal strong{color:#d5dfec}.help-modal .full{justify-content:center;margin-top:20px}@keyframes pulse{50%{opacity:.3}}@media(min-width:1500px){main{grid-template-columns:280px minmax(400px,1fr) 290px}.sidebar{padding-left:25px;padding-right:25px}.board-workspace{padding-left:36px;padding-right:36px}}@media(max-width:1130px){main{grid-template-columns:230px minmax(380px,1fr);height:auto;overflow:visible}.sidebar{overflow:visible}.board-workspace{height:calc(100vh - 76px);min-height:650px}.right-sidebar{grid-column:2;border-left:0;display:grid;grid-template-columns:1fr 1fr;gap:16px;border-top:1px solid #293444}.right-sidebar>.section-kicker,.right-title{display:none}.right-sidebar>section,.right-sidebar>.summary-counts{margin:0}.right-sidebar .method-note{grid-column:1/-1}.left-sidebar{grid-row:1/3}.board-workspace{padding:23px 20px 18px}.local-badge{display:none}.board-scroll{min-height:180px}.board-stats{gap:8px}.board-bottom{gap:8px}.legend{gap:7px}.right-sidebar .inspector{border:0;padding-top:0}.right-sidebar .move-list{grid-column:1/-1}.banner{gap:8px}}@media(prefers-reduced-motion:reduce){:global(button){transition:none}.working{animation:none}}
  .board-comparison{padding:10px 12px;border:1px solid #355849;border-radius:7px;background:#192b27;margin-top:12px}.board-comparison>strong{font-size:11px;color:#a5dbbf}.board-comparison p{margin:5px 0 0}.board-comparison details{margin-top:7px}
  /* Keep the center dedicated to the board at every desktop window size. */
  .app-shell{--right-sidebar-width:264px}.app-header{height:64px;padding:0 20px}main{height:calc(100vh - 64px)}
  .board-workspace{padding:10px 18px 6px}.board-toolbar{flex-shrink:0;padding:0 0 9px;min-height:45px;gap:8px}.board-heading{min-width:0}.board-heading h2{font-size:13px;line-height:1.4;margin:0 0 3px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.board-stats{gap:9px;align-items:center;font-size:9px;white-space:nowrap}.board-stats .status-badge{padding:0;background:none;border:0;font-size:9px}.toolbar-actions{flex-shrink:0}.board-scroll{padding:15px 12px;min-height:100px}.board-bottom{padding:7px 10px;flex-shrink:0;min-height:35px}.analysis-footer{padding:8px 0 0;min-height:23px;flex-shrink:0;gap:8px;font-variant-numeric:tabular-nums}.analysis-footer>span:first-child{white-space:nowrap}.navigation-help{white-space:nowrap;text-align:right;color:#7e93ac}
  .right-sidebar .edit-dock{margin:0 0 20px;padding:13px;position:sticky;top:0;z-index:4}.right-sidebar .edit-tools{display:grid;grid-template-columns:repeat(5,30px);gap:5px}.right-sidebar .edit-tools>button{width:30px;height:30px}.right-sidebar .edit-dock-title strong{line-height:1.5}.right-sidebar .paint-toggle{grid-column:4 / span 2;grid-row:3;margin:0;width:auto;font-size:10px}.right-sidebar .banner.review{padding:12px;gap:10px;margin:0 0 20px}.right-sidebar .banner.review>div{min-width:0;flex-basis:100%}.right-sidebar .banner.review>button{flex:1;justify-content:center}.right-sidebar .banner.review>span{display:none}
  .toast{position:fixed;right:12px;bottom:14px;width:calc(var(--right-sidebar-width) - 24px);z-index:12;display:flex;align-items:flex-start;gap:9px;padding:13px 11px;border:1px solid #3d826a;border-radius:9px;background:#1a342e;box-shadow:0 8px 28px #0008;color:#c4ecdb;max-height:220px;overflow:auto}.toast.toast-error{background:#39232d;border-color:#ab526b;color:#f4bac9}.toast p{font-size:11px;line-height:1.6;margin:0;flex:1;overflow-wrap:anywhere}.toast-icon{font-size:15px;line-height:1.5}.toast>button{background:transparent;border:0;color:inherit;font-size:19px;padding:0 2px;line-height:1.2}
  @media(min-width:1500px){.app-shell{--right-sidebar-width:290px}.board-workspace{padding:12px 22px 6px}}
  @media(max-width:1499px){.navigation-help{display:none}}
  @media(max-width:1130px){.app-shell{--right-sidebar-width:230px}main{grid-template-columns:190px minmax(300px,1fr) 230px;height:calc(100vh - 64px);overflow:hidden}.sidebar{padding:18px 13px;overflow-y:auto}.left-sidebar{grid-row:auto}.right-sidebar{display:block;grid-column:auto;border-left:1px solid #26303c;border-top:0}.right-sidebar>.section-kicker,.right-sidebar .right-title{display:block}.right-sidebar>section{margin-top:20px}.right-sidebar .edit-dock{margin-top:0}.board-workspace{height:auto;min-height:0;padding:10px 10px 6px}.board-stats .detail-stat{display:none}.board-scroll{min-height:100px}.header-actions{gap:7px}.app-header{padding:0 14px}.quiet{padding:8px}.right-sidebar .inspector{margin-top:20px;border-top:1px solid #2c3847;padding-top:20px}.board-bottom{flex-wrap:wrap}.analysis-footer{font-size:8px}}
</style>

















