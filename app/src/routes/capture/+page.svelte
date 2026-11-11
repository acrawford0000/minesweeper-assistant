<script lang="ts">
  import {onMount} from 'svelte';
  import {invoke} from '@tauri-apps/api/core';
  let preview=$state<{image:string;bounds:{width:number;height:number}}|null>(null);
  let error=$state('');let busy=$state(false);
  let start=$state<{x:number;y:number}|null>(null);let end=$state<{x:number;y:number}|null>(null);let dragging=$state(false);
  const selection=$derived(start&&end?{x:Math.min(start.x,end.x),y:Math.min(start.y,end.y),width:Math.abs(end.x-start.x),height:Math.abs(end.y-start.y)}:null);
  function point(e:PointerEvent){return{x:Math.max(0,Math.min(1,e.clientX/window.innerWidth)),y:Math.max(0,Math.min(1,e.clientY/window.innerHeight))};}
  function down(e:PointerEvent){if(e.button!==0||busy||!preview)return;error='';start=point(e);end=start;dragging=true;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);}
  function move(e:PointerEvent){if(dragging)end=point(e);}
  function up(e:PointerEvent){if(!dragging)return;end=point(e);dragging=false;const node=e.currentTarget as HTMLElement;if(node.hasPointerCapture(e.pointerId))node.releasePointerCapture(e.pointerId);}
  async function accept(){if(!selection||busy)return;busy=true;try{await invoke('finish_capture_selection',{selection});}catch(cause){error=String(cause);busy=false;}}
  async function cancel(){if(busy)return;try{await invoke('cancel_capture_selection');}catch(cause){error=String(cause);}}
  function keyboard(event:KeyboardEvent){if(event.key==='Escape'){event.preventDefault();void cancel();}else if(event.key==='Enter'&&selection){event.preventDefault();void accept();}}
  onMount(()=>{void invoke<typeof preview>('capture_preview').then(result=>preview=result).catch(cause=>error=String(cause));});
</script>
<svelte:head><title>Select Minesweeper capture area</title></svelte:head>
<svelte:window onkeydown={keyboard}/>
<main onpointerdown={down} onpointermove={move} onpointerup={up} role="application" aria-label="Drag to select the Minesweeper board and mine counter">
  {#if preview}<img src={preview.image} alt="Frozen desktop preview for selecting a capture area" draggable="false"/>{:else}<div class="loading">Preparing screen preview…</div>{/if}
  {#if selection}<div class="selection" style:left={`${selection.x*100}%`} style:top={`${selection.y*100}%`} style:width={`${selection.width*100}%`} style:height={`${selection.height*100}%`}><span>{Math.round(selection.width*(preview?.bounds.width??0))} × {Math.round(selection.height*(preview?.bounds.height??0))}</span></div>{/if}
</main>
<div class="instructions"><div><strong>Select your board</strong><span>Drag around the complete board and mine counter. Enter confirms · Esc cancels.</span></div><button onclick={cancel} disabled={busy}>Cancel</button><button class="confirm" onclick={accept} disabled={!selection||busy}>{busy?'Applying…':'Use this area'}</button>{#if error}<p role="alert">{error}</p>{/if}</div>
<style>
  :global(body){margin:0;overflow:hidden;background:#101820;color:#e5f4ff;font-family:'Segoe UI',sans-serif} :global(*){box-sizing:border-box}main{position:fixed;inset:0;cursor:crosshair;touch-action:none;user-select:none}img{width:100%;height:100%;object-fit:fill;pointer-events:none;display:block}.loading{display:grid;place-items:center;height:100%;font-size:20px}.selection{position:absolute;border:2px solid #40caff;box-shadow:0 0 0 99999px #0008;pointer-events:none}.selection>span{position:absolute;left:0;bottom:-25px;padding:4px 7px;background:#112f42;color:#bef0ff;font-size:11px;white-space:nowrap;border-radius:3px}.instructions{position:fixed;top:18px;left:50%;transform:translateX(-50%);display:flex;align-items:center;gap:13px;flex-wrap:wrap;width:max-content;max-width:calc(100vw - 36px);padding:15px 18px;background:#142431f5;border:1px solid #45728d;border-radius:10px;box-shadow:0 8px 32px #0006}.instructions>div{display:flex;flex-direction:column;gap:5px}.instructions strong{font-size:15px}.instructions span{font-size:12px;color:#a6c4d7}.instructions button{font:inherit;font-size:12px;border:1px solid #48667b;border-radius:6px;padding:9px 13px;background:#253c4d;color:#d3e8f5;cursor:pointer}.instructions button.confirm{background:#41c6ff;color:#072337;border-color:#41c6ff;font-weight:650}button:disabled{opacity:.45;cursor:default}.instructions p{margin:0;flex-basis:100%;color:#ffafbc;font-size:12px}
</style>
