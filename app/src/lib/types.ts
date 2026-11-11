export interface Board {
  rows: number;
  cols: number;
  cells: number[];
  totalMines: number | null;
  trustFlags: boolean;
  revision: number;
}
export interface RiskDisplay {rounded:number;color:string;highlight:'none'|'safe'|'mine'|'best'|'worst';muted:boolean}
export interface Odds { probability: number | null; kind: 'safe' | 'mine' | 'risk' | 'unknown'; reason: string; numerator: string | null;display?:RiskDisplay|null }
export interface EfficiencyAction {kind:'flag'|'chord'|'reveal';cell:number;clears:number[];explanation:string}
export interface EfficiencyPlan {status:'safe'|'guess'|'done'|'unavailable';actions:EfficiencyAction[];targets:number[];clicks:number;knownSafe:number;projectedClicks:number;directClicks:number;optimal:boolean;message:string}
export interface Analysis { revision: number; status: string; message: string; odds: Odds[]; solutions: string | null; conflicts: number[]; nodes: number; elapsedMs: number; components: number; cachedComponents?:number;pooledCells?:number; recommendations:number[];efficiency?:EfficiencyPlan|null }
export interface Rect {x:number;y:number;width:number;height:number}
export interface ScreenRect {x:number;y:number;width:number;height:number}
export interface CaptureStatus {region:ScreenRect|null;hotkey:string|null;configuredHotkey:string;hotkeyEnabled:boolean;shortcutError:string|null;selecting:boolean}
export interface CapturedRegion {region:ScreenRect;image:string}
export interface CounterReading {value:number;text:string;rect:Rect;flagCount:number;totalMines:number|null}
export interface BoardComparison {kind:'same'|'progress'|'different'|'uncertain';preservedKnown:number;newlyRevealed:number;newFlags:number;changedKnown:number;unchangedCovered:number;message:string}
export interface Scan {rows:number;cols:number;cells:number[];confidence:number[];notes:string[];rect:Rect;message:string;counter?:CounterReading|null;clueColors?:Array<string|null>;autoAccept?:boolean;reviewCells?:number[];reviewReasons?:string[];comparison?:BoardComparison}
export interface Session {version:1;board:Board;image:string|null;scan:Scan|null;reviewed:boolean;name:string}

export function readSession(text: string): Session {
  const value = JSON.parse(text);
  const b = value?.board;
  if (value?.version !== 1 || !b || !Number.isInteger(b.rows) || !Number.isInteger(b.cols) || b.rows < 1 || b.cols < 1 || b.rows > 100 || b.cols > 100 || b.rows*b.cols>2500 || !Array.isArray(b.cells) || b.cells.length !== b.rows*b.cols || b.cells.some((v:unknown)=>typeof v!=='number' || !Number.isInteger(v) || v < -5 || v > 8) || (b.totalMines !== null && (!Number.isInteger(b.totalMines) || b.totalMines < 0 || b.totalMines > b.cells.length)) || typeof b.trustFlags!=='boolean') {
    throw new Error('This is not a supported Minesweeper Assistant session.');
  }
  if (value.image !== null && (typeof value.image !== 'string' || !/^data:image\/(png|jpeg|webp);base64,/.test(value.image) || value.image.length>28_000_000)) throw new Error('The saved image is invalid or too large.');
  // Recognition metadata is optional and never trusted as board state.
  const scan = value.scan && Array.isArray(value.scan.confidence) && value.scan.confidence.length===b.cells.length && Array.isArray(value.scan.notes) && value.scan.notes.length===b.cells.length ? value.scan : null;
  return {version:1,board:{...b,cells:b.cells.map((v:number)=>v===-3?-2:v),trustFlags:true,revision:0},image:value.image,scan,reviewed:!!value.reviewed,name:typeof value.name==='string'?value.name.slice(0,120):'Saved board'};
}
