/* Actual pinned renderer parsers, components, event handlers and reconciliation;
 * UI/DOM/clipboard/browser producers are closed VM stubs before callbacks. */
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),ts=require(path.join(root,'.qa/v1/node_modules/typescript'));
const context=vm.createContext({}, {codeGeneration:{strings:false,wasm:false}});
function run(code){return new vm.Script(code).runInContext(context,{timeout:2000});}
run(fs.readFileSync(path.join(__dirname,'sandbox.js'),'utf8'));
run(`
const ui={cx:(...xs)=>xs.filter(Boolean).join(' '),Hint:'Hint',HintBar:'HintBar',Kbd:'Kbd',ViewHeader:'ViewHeader'};
modules['host/renderer/ui/primitives']={exports:ui};
modules['host/renderer/ui/Markdown']={exports:{Markdown:'Markdown'}};
modules['host/renderer/colorConverter']={exports:{buildColorFormatRows:()=>{throw Error('unowned producer')},buildColorConversionResults:()=>{throw Error('unowned producer')}}};
const pendingTimers=new Map();let timerSerial=0;
function setTimeout(fn){const id=++timerSerial;pendingTimers.set(id,fn);return id;}
function clearTimeout(id){pendingTimers.delete(id);}
function flushTimers(){const fns=[...pendingTimers.values()];pendingTimers.clear();for(const fn of fns)fn();}
function findNodes(node,pred){if(!node||typeof node!=='object')return[];return [...(pred(node)?[node]:[]),...[node.props?.children].flat(Infinity).flatMap(n=>findNodes(n,pred))];}
function allText(node){if(node===undefined||node===null||typeof node==='boolean')return '';if(typeof node!=='object')return String(node);return [node.props?.children].flat(Infinity).map(allText).join(' ');}
`);
for(const file of ['detail-runtime','list-runtime']){
 let source=fs.readFileSync(path.join(root,'.qa/host/src/renderer/src/raycast-api',file+'.tsx'),'utf8');
 if(file==='list-runtime')source+='\nexport const __test={parseListRows,metadataFromListDetail,clientSideFilter,groupBySection};';
 const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true}}).outputText;
 run(`factories['host/renderer/src/raycast-api/${file}']=function(exports,require,module){${js}\n};`);
}
const views=JSON.parse(fs.readFileSync(path.join(root,'.qa/production-views.json')));
run(`const views=${JSON.stringify(views)};const list=requireModule('host/renderer/src/raycast-api/list-runtime');const detail=requireModule('host/renderer/src/raycast-api/detail-runtime');let received=[],invoked=[],current=views[0];const changed=text=>received.push(text);const props=()=>({root:current.root,title:current.title,commandName:'coins',actions:current.actions,onBack:()=>{},onRunPrimaryAction:(id)=>invoked.push(id),onOpenActions:()=>{},onSearchTextChanged:changed,onLoadMore:()=>{}});`);
let checks=0;function check(code){assert.equal(run(code),true,code);checks++;}
check(`list.__test.parseListRows(views[0].root).length===2`);
check(`list.__test.parseListRows(views[2].root).map(r=>r.title).join(',')==='GPU 2,GPU 10,Synthetic GPU'`);
check(`list.__test.parseListRows(views[0].root)[0].accessories[0].text==='$3.000'`);
check(`list.__test.metadataFromListDetail(list.__test.parseListRows(views[1].root)[0].detail).children.length===27`);
run(`engine.mount(()=>list.ListRuntime(props()));`);
check(`findNodes(engine.tree,n=>typeof n.props?.className==='string'&&n.props.className.includes('grid-cols-')).length===0`);
run(`current=views[1];engine.render();`);
check(`findNodes(engine.tree,n=>typeof n.props?.className==='string'&&n.props.className.includes('grid-cols-')).length===1`);
run(`const input=()=>findNodes(engine.tree,n=>n.type==='input')[0];for(const value of ['A','B','A']){input().props.onChange({target:{value}});engine.render();}`);
check(`input().props.value==='A'&&pendingTimers.size===1`);
run(`current=views[0];engine.render();`);check(`input().props.value==='A'`);
run(`flushTimers();`);check(`received.join(',')==='A'`);
run(`current=views[1];engine.render();`);check(`input().props.value==='A'`);
run(`input().props.onChange({target:{value:'TestAlgo'}});engine.render();`);
check(`list.__test.clientSideFilter(list.__test.parseListRows(current.root),'TestAlgo').length===1`);
check(`input().props.value==='TestAlgo'`);
run(`input().props.onChange({target:{value:''}});engine.render();const button=findNodes(engine.tree,n=>n.type==='button'&&n.props['data-idx']===0)[0];button.props.onClick();`);
check(`invoked[0]===current.actions[0].id`);
run(`const metadata=list.__test.metadataFromListDetail(list.__test.parseListRows(views[1].root)[0].detail);const rendered=metadata.children.filter(n=>n.type.endsWith('Label')).map(node=>allText(detail.MetadataItem({node}))).join('|');`);
check(`rendered.includes('USD Price 24h')&&rendered.includes('Coin EstimateAvg30d')&&rendered.includes('Network Difficulty')&&rendered.includes('EmissionUSD')`);
run(`const gpuMetadata=list.__test.metadataFromListDetail(list.__test.parseListRows(views[2].root)[0].detail);const gpuText=gpuMetadata.children.filter(n=>n.type.endsWith('Label')).map(node=>allText(detail.MetadataItem({node}))).join('|');`);
check(`gpuText.includes('Highest Profit 24H')&&gpuText.includes('$1.01')&&gpuText.includes('-$0.00')`);
check(`typeof process==='undefined'&&typeof window==='undefined'`);
fs.writeFileSync(path.join(root,'evidence/renderer-qa.json'),JSON.stringify({commit:'ea100cee3f69711de12485ba061950c5fc65258f',checks,producers:'Actual list-runtime.tsx + detail-runtime.tsx; UI/color/DOM/IO producers stubbed; no actual GUI',patchRequired:false,search:'A/B/A before debounce; late root reconciliation preserves draft; local filter and action selection tested'},null,2)+'\n');
console.log(`Pinned renderer: ${checks} parser/component/reconciliation checks passed.`);
