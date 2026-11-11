// One observer and one layout pass for the grid, rather than thousands of
// independent read/write callbacks. Equal labels share a width measurement.
const nodes=new Set<HTMLElement>();
let observer:ResizeObserver|undefined;
let frame=0;
function schedule(){if(!frame)frame=requestAnimationFrame(fitAll);}
function fitAll(){
  frame=0;
  const metrics=new Map<Element,{width:number;labels:Map<string,number>}>();
  const updates:Array<[HTMLElement,string]>=[];
  for(const node of nodes){
    if(!node.isConnected||!node.textContent)continue;
    const group=node.closest('.board-wrap')??node.parentElement!;
    let metric=metrics.get(group);
    if(!metric){metric={width:Math.max(1,(node.parentElement?.clientWidth??0)-2),labels:new Map()};metrics.set(group,metric);}
    const label=node.textContent;
    let scale=metric.labels.get(label);
    if(scale===undefined){scale=Math.min(1,metric.width/Math.max(1,node.scrollWidth));metric.labels.set(label,scale);}
    updates.push([node,`scaleX(${scale})`]);
  }
  // Read every required width before writing styles, avoiding layout thrashing.
  for(const [node,value] of updates)if(node.style.transform!==value)node.style.transform=value;
}
export function fitCellText(node:HTMLElement){
  observer??=new ResizeObserver(schedule);nodes.add(node);observer.observe(node);schedule();
  return {destroy(){observer?.unobserve(node);nodes.delete(node);if(!nodes.size&&frame){cancelAnimationFrame(frame);frame=0;}}};
}
