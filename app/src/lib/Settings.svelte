<script lang="ts">
  import {controlActions,defaultBindings,keyBinding,reservedBinding,bindingLabel,bindingOwner,type Bindings} from './controls';
  import {invoke} from '@tauri-apps/api/core';
  import type {CaptureStatus} from './types';
  let {bindings=$bindable(),fontScale=$bindable(),captureStatus=$bindable(),onclose,focusModal}:{bindings:Bindings;fontScale:number;captureStatus:CaptureStatus;onclose:()=>void;focusModal:(node:HTMLElement)=>{destroy:()=>void}}=$props();
  let recording=$state<string|null>(null);
  let recordingCapture=$state(false);let savingCapture=$state(false);let captureMessage=$state('');
  let message=$state('');
  function capture(event:KeyboardEvent){
    if(!recording&&!recordingCapture)return;
    event.preventDefault();event.stopImmediatePropagation();
    if(event.key==='Escape'){recording=null;recordingCapture=false;message='Binding canceled.';captureMessage='Binding canceled.';return;}
    if(event.repeat||event.isComposing)return;
    const key=keyBinding(event);if(!key)return;
    if(recordingCapture){if(!savingCapture)void updateCaptureHotkey(key);return;}
    if(!recording)return;
    if(key===captureStatus.configuredHotkey){message='That key is assigned to screen capture. Change the capture hotkey first.';return;}
    if(reservedBinding(key)){message='That key is reserved for closing, moving focus, clipboard, or system shortcuts. Choose another.';return;}
    const owner=bindingOwner(bindings,key);
    if(owner&&owner!==recording){message=`${bindingLabel(key)} is already assigned to ${controlActions.find(a=>a.id===owner)?.label}. Remove that binding first, or choose another key.`;return;}
    if(owner===recording){message='This action already has that binding.';recording=null;return;}
    bindings={...bindings,[recording]:[...bindings[recording],key]};recording=null;message=`Added ${bindingLabel(key)}.`;
  }
  async function updateCaptureHotkey(key:string|null){
    if(key&&bindingOwner(bindings,key)){captureMessage='That key is already used by an editing control. Remove that binding or choose another combination.';return;}
    savingCapture=true;try{captureStatus=await invoke<CaptureStatus>('set_capture_hotkey',{hotkey:key});recordingCapture=false;captureMessage=key?`Capture key updated.${captureStatus.hotkeyEnabled?'':' Still paused; enable it when ready.'}`:'Global capture hotkey disabled.';}catch(cause){captureMessage=String(cause);}finally{savingCapture=false;}
  }
  async function toggleCaptureHotkey(){if(savingCapture)return;savingCapture=true;recordingCapture=false;try{captureStatus=await invoke<CaptureStatus>('set_capture_hotkey_enabled',{enabled:!captureStatus.hotkeyEnabled});captureMessage=captureStatus.hotkeyEnabled?'Global capture hotkey enabled. Turn it off before typing in other apps.':'Global capture hotkey disabled. The saved key is kept for next time.';}catch(cause){captureMessage=String(cause);}finally{savingCapture=false;}}
  function remove(id:string,key:string){bindings={...bindings,[id]:bindings[id].filter(k=>k!==key)};message=`Removed ${bindingLabel(key)}.`;}
  function reset(){bindings=defaultBindings();fontScale=100;recording=null;recordingCapture=false;message='Default controls and font size restored.';void updateCaptureHotkey('y');}
</script>

<svelte:window onkeydowncapture={capture}/>
<div class="modal-backdrop">
  <div class="modal settings-modal" use:focusModal tabindex="-1" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <header><div><span class="kicker">MAKE IT YOURS</span><h2 id="settings-title">Settings</h2></div><button class="close" aria-label="Close settings" onclick={onclose}>×</button></header>
    <div class="settings-content">
    <p class="session-note">For this open app only. Changes apply immediately and reset when you close the app. No settings file is created.</p>
    <section class="font-settings" aria-labelledby="font-title">
      <div class="section-heading"><h3 id="font-title">Board font size</h3><output for="board-font-size">{fontScale}%</output></div>
      <input id="board-font-size" aria-label="Board font size" type="range" min="70" max="140" step="5" bind:value={fontScale}/>
      <p>Changes clues, percentages, and symbols inside board cells. Cell size, zoom, and the rest of the app stay the same.</p>
      <div class="font-preview" style:--board-font-scale={fontScale/100} aria-label="Board font preview"><span class="preview-cell clue">4</span><span class="preview-cell risk">12<span>%</span></span><span class="preview-cell flag">⚑</span></div>
    </section>
    <section aria-labelledby="capture-title">
      <h3 id="capture-title">Screen capture hotkey</h3><p>Captures and analyzes the area you already selected. It never changes the area. Turn it off before typing messages; while off, the key types normally and the capture button still works.</p>
      <div class="capture-hotkey"><kbd>{bindingLabel(captureStatus.configuredHotkey)}</kbd><span class="hotkey-state">{captureStatus.hotkeyEnabled?'On':'Paused'}</span><button class="add-key" disabled={savingCapture} onclick={()=>{recording=null;recordingCapture=!recordingCapture;captureMessage='';}}>{recordingCapture?'Listening…':'Change hotkey'}</button><button class="add-key" disabled={savingCapture} onclick={()=>void toggleCaptureHotkey()}>{captureStatus.hotkeyEnabled?'Disable':'Enable'}</button></div>
      <p class="capture-feedback" role="status">{recordingCapture?'Press a letter, digit, function key, or combination. Esc cancels. ':''}{captureMessage||captureStatus.shortcutError||'Default key: Y. Starts paused at each launch. Disabling keeps your binding so Enable restores it.'}</p>
    </section>
    <section aria-labelledby="controls-title">
      <h3 id="controls-title">Keyboard controls</h3>
      <p>Choose <strong>Add key</strong>, then press a key or combination. Keep several keys for one action, or remove a binding with ×. For example, add Q to the revealed blank action and keep 0 too.</p>
      <div class="capture-message" role="status" aria-live="polite">{recording?`Listening for ${controlActions.find(a=>a.id===recording)?.label}. Press a key; Esc cancels.`:message||'Select a board square before using an editing shortcut.'}{#if recording&&message}<span>{message}</span>{/if}</div>
      <div class="binding-list">
        {#each controlActions as action}
          <div class="binding-row" class:listening={recording===action.id}>
            <strong>{action.label}</strong>
            <div class="keys">{#each bindings[action.id] as key}<button class="key-chip" aria-label={`Remove ${bindingLabel(key)} from ${action.label}`} title="Remove binding" onclick={()=>remove(action.id,key)}><kbd>{bindingLabel(key)}</kbd><span>×</span></button>{/each}{#if !bindings[action.id].length}<span class="unbound">No keys assigned</span>{/if}</div>
            <button class="add-key" aria-label={`Add key for ${action.label}`} aria-pressed={recording===action.id} onclick={()=>{recordingCapture=false;message='';recording=recording===action.id?null:action.id;}}>{recording===action.id?'Listening…':'Add key'}</button>
          </div>
        {/each}
      </div>
      <p class="fixed-controls">Mouse: click to select, right-click to toggle a flag, wheel to zoom, Ctrl + drag or middle-drag to pan. Esc stops paint mode. These mouse controls stay fixed.</p>
    </section>
    </div>
    <footer><button class="secondary" onclick={reset}>Restore defaults</button><button class="primary" onclick={onclose}>Done</button></footer>
  </div>
</div>

<style>
  .capture-hotkey{display:flex;align-items:center;gap:9px;flex-wrap:wrap}.capture-hotkey>kbd{padding:7px 10px;background:#21374a;border:1px solid #3e617d;border-radius:6px;color:#bfe6ff;font-size:12px}.capture-feedback{font-size:11px;color:#b5d4ea}
  .modal-backdrop{position:fixed;inset:0;background:#050a12c9;backdrop-filter:blur(5px);display:flex;align-items:center;justify-content:center;z-index:20;padding:24px}
  .settings-modal{display:flex;flex-direction:column;width:700px;max-width:100%;max-height:90vh;overflow:hidden;background:#17212d;border:1px solid #3c4b5e;border-radius:14px;padding:25px;box-shadow:0 20px 80px #0006;color:#e7edf4}.settings-content{overflow:auto;min-height:0;padding-right:8px;margin-top:10px}header,footer{flex-shrink:0}
  header{display:flex;align-items:center;justify-content:space-between}.kicker{font-size:9px;letter-spacing:1.5px;color:#91a4ba}h2{font-size:23px;margin:7px 0 0}h3{font-size:14px;margin:0}p{font-size:12px;line-height:1.6;color:#9fb1c7}.session-note{padding:12px 14px;border:1px solid #4b514c;border-radius:8px;background:#252d2d;color:#b7c7ba;margin:18px 0 24px}.close{background:transparent;color:#aebed2;border:1px solid #3a495c;border-radius:6px;font-size:23px;width:32px;height:32px}
  section+section{margin-top:24px;padding-top:23px;border-top:1px solid #354151}.section-heading{display:flex;align-items:center;justify-content:space-between}output{font-size:13px;color:#edb17d}input[type=range]{width:100%;accent-color:#edb17d;margin:15px 0 0}.font-settings>p{margin-top:8px}.font-preview{display:flex;gap:7px}.preview-cell{width:48px;height:48px;display:flex;align-items:center;justify-content:center;background:#29394d;border:1px solid #42526a;border-radius:5px;font-weight:700;line-height:1}.clue{background:#17212d;color:#ee88ff;font-size:calc(21px * var(--board-font-scale))}.risk{color:#36f9a3;font-size:calc(21px * var(--board-font-scale));letter-spacing:-.5px}.risk span{font-size:.62em;align-self:center}.flag{color:#e6bc82;font-size:calc(20px * var(--board-font-scale))}
  .capture-message{padding:10px 12px;margin:13px 0;color:#b4d6f5;background:#192e40;border:1px solid #365573;border-radius:7px;font-size:11px;line-height:1.6;min-height:39px}.capture-message span{display:block;color:#f0c18b}.binding-list{border:1px solid #334256;border-radius:8px;overflow:hidden}.binding-row{display:grid;grid-template-columns:165px 1fr 85px;align-items:center;gap:12px;padding:10px 12px;background:#141e2a}.binding-row+.binding-row{border-top:1px solid #2c394a}.binding-row.listening{background:#21384d}.binding-row>strong{font-size:12px;font-weight:500}.keys{display:flex;gap:6px;flex-wrap:wrap}.key-chip{display:inline-flex;align-items:center;gap:7px;border:1px solid #485870;border-radius:5px;background:#263448;color:#d1e0f1;padding:5px 7px;font-size:11px}.key-chip:hover{border-color:#d39e76}.key-chip span{color:#92a6c0}kbd{font:inherit}.add-key{background:#243448;color:#b9d7f6;border:1px solid #435973;border-radius:5px;padding:7px;font-size:11px}.unbound{font-size:10px;color:#8093ac}.fixed-controls{font-size:11px;margin-bottom:0}footer{display:flex;justify-content:space-between;gap:12px;margin-top:24px}.primary,.secondary{border-radius:6px;padding:9px 15px;font-size:12px;border:1px solid #44556b}.primary{color:#271e17;background:#edb17d;border-color:#edb17d}.secondary{color:#d0deef;background:#253345}@media(max-width:650px){.binding-row{grid-template-columns:1fr 85px}.keys{grid-row:2;grid-column:1/-1}.binding-row>.add-key{grid-column:2;grid-row:1}.settings-modal{padding:18px}}
</style>
