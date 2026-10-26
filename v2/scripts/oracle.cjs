/* Executes the actual V1 TSX, hook and locked utils in a closed VM. Imports,
 * Cache, React and fetch producers are installed before any callback executes. */
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),repo=path.dirname(root);
const ts=require(path.join(root,'.qa/v1/node_modules/typescript'));
const sourceMap=JSON.parse(fs.readFileSync(path.join(root,'.qa/v1/node_modules/@raycast/utils/dist/module.js.map'),'utf8'));
const sources={};
for(let i=0;i<sourceMap.sources.length;i++) {
 const name=sourceMap.sources[i].replace(/^\.\.\/src\//,'').replace(/^\.\.\//,'').replace(/^src\//,'');
 sources['utils/'+name.replace(/\.tsx?$/,'')]=sourceMap.sourcesContent[i];
}
for(const f of ['coins','gpu_estimates','useHashrate','types']) sources['src/'+f]=fs.readFileSync(path.join(repo,'src',f+(f==='coins'||f==='gpu_estimates'?'.tsx':'.ts')),'utf8');
sources['dequal/lite']=fs.readFileSync(path.join(root,'.qa/v1/node_modules/dequal/lite/index.js'),'utf8');
const sandbox=fs.readFileSync(path.join(__dirname,'sandbox.js'),'utf8');
function create() {
 const context=vm.createContext({}, {codeGeneration:{strings:false,wasm:false}});
 new vm.Script(sandbox).runInContext(context,{timeout:1500});
 for(const [name,source] of Object.entries(sources)) {
  if(source==null) continue;
  const compiled=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true}}).outputText;
  new vm.Script(`factories[${JSON.stringify(name)}]=function(exports,require,module){${compiled}\n};`,{filename:name}).runInContext(context,{timeout:1500});
 }
 return context;
}
function run(c,script) { return new vm.Script(script).runInContext(c,{timeout:1500}); }
async function flush(c) {
 for(let i=0;i<50;i++) { await Promise.resolve(); run(c,'if(engine.dirty) engine.render();'); }
 run(c,'if(engine.dirty) throw Error("VM did not settle");');
}
function rows(c) { return JSON.parse(run(c,'JSON.stringify(project(engine.tree))')); }
const coinFields=Array.from(fs.readFileSync(path.join(repo,'src/types.ts'),'utf8').split('export type Coin = {')[1].split('};')[0].matchAll(/^  (\w+): string;/gm),m=>m[1]);
const coins=[Object.fromEntries(coinFields.map((k,i)=>[k,['TEST','Synthetic Coin','TestAlgo'][i]??`${i}.000`])),{coin:'OTHER',name:'Second Synthetic',algo:'AnotherAlgo',usdPrice:'0.0000000100'}];
const estimate={device:{name:'Synthetic GPU',brand:'Owned Test Brand'},profit:{coin:'TEST',revenueUSD:1.005,profitUSD:-0.004},revenue:{coin:'OTHER',revenueUSD:1234.567,profitUSD:2.675},profit24:{coin:'TEST',revenueUSD24:1e21,profitUSD24:-0},revenue24:{coin:'OTHER',revenueUSD24:0.005,profitUSD24:0.004}};
const gpu={z:estimate,10:{...estimate,device:{name:'GPU 10',brand:'Test'}},2:{...estimate,device:{name:'GPU 2',brand:'Test'}}};
async function launch(c,command,power,data) {
 run(c,`fixtureData=${JSON.stringify(data)}; engine.mount(()=>requireModule('src/${command}').default({arguments:{power_cost:${JSON.stringify(power)}}}));`);
 await flush(c);
}
(async()=>{
 const output={epoch:run(create(),'Date.now()'),producer:'actual V1 TSX/useHashrate + locked @raycast/utils source map; closed VM; synthetic fixtures only',views:[],numeric:[],currency:[],lifecycle:[],mime:[]};
 for(const [command,input] of [['coins',coins],['gpu_estimates',gpu]]) {
  const c=create();assert.equal(run(c,"sha1('abc')"),'a9993e364706816aba3e25717850c26c9cd0d89d');await launch(c,command,'.5',input);
  assert.equal(run(c,'requests.length'),1); assert.equal(run(c,'treeActions().map(x=>x.props.title).join(",")'),'Toggle Details,Refresh,Toggle Details,Refresh'+(command==='gpu_estimates'?',Toggle Details,Refresh':''));
  const firstRows=rows(c);const detail=command!=='coins';output.views.push({command,input,detail,section:run(c,'engine.tree.props.children.props.title'),rows:firstRows});
  assert.equal(run(c,'engine.tree.props.searchBarPlaceholder'),command==='coins'?'Search coins':'Search GPUs');
  run(c,'treeActions()[0].props.onAction()');await flush(c);output.views.push({command,input,detail:!detail,section:run(c,'engine.tree.props.children.props.title'),rows:rows(c)});assert.equal(run(c,'requests.length'),1);
  run(c,'treeActions()[1].props.onAction()');await flush(c);assert.equal(run(c,'requests.length'),2);
  await launch(c,command,'.5',input);
  const second=run(c,'requests.length');assert.equal(second,command==='coins'?2:3);
  await launch(c,command,'.5',input);assert.equal(run(c,'requests.length'),second);
  output.lifecycle.push({command,firstLaunch:1,toggleExtra:0,manualRefreshExtra:1,secondLaunchExtra:command==='coins'?0:1,thirdLaunchExtra:0,request:JSON.parse(run(c,'JSON.stringify(requests[0])'))});
  // Successful empty results trigger source refresh on each launch.
  const empty=create();await launch(empty,command,'.5',command==='coins'?[]:{});await launch(empty,command,'.5',command==='coins'?[]:{});assert.equal(run(empty,'requests.length'),2);
 }
 const inputs=['','0','-0','0.0','  ','0x0','NaN','abc','inf','+0x1','1_0','\u00850.5','1',' .5 ','+0.5','1e-2','0x1','0b1','0o1','\ufeff.25','5e-324','Infinity','-Infinity','1.01','-0.1','1e309','5e-325','0X1','0B1','0O1','0.','1.','+.5','+1e-1','\u20280.5\u2029'];
 for(const input of inputs) {
  const c=create();await launch(c,'gpu_estimates',input,{});
  const error=run(c,'failures[0]??null');output.numeric.push({input,error});assert.equal(run(c,'requests.length'),error?0:1);
 }
 // Exercise the actual nested formatter from GPU TSX via its rendered metadata.
 for(const input of [null,true,false,'-0','1.005','-1.005',' 2.675 ','0.0049','0.0051',9.995,99.995,-1.005,-2.675,-99.995,0.0005,0.00005,1e-6,1e-5,1e-4,1e22,1e-2,1e-3,1e-10,'1e309','-1e309','abc',[],'1.005','NaN','Infinity','-Infinity','',0,-0,1.005,2.675,-0.004,1234.567,0.005,0.004,1e-7,1e21,1e23,1e100,Number.MAX_VALUE,Number.MIN_VALUE,-99.995]) {
  const c=create();const value=structuredClone(estimate);value.profit.revenueUSD=input;await launch(c,'gpu_estimates','.5',{test:value});
  output.currency.push({input,expected:rows(c)[0].metadata[2].text});
 }
 const c=create();const missing=structuredClone(estimate);delete missing.profit.revenueUSD;await launch(c,'gpu_estimates','.5',{test:missing});output.currency.push({expected:rows(c)[0].metadata[2].text});
 for(const input of ['application/json','text/json','application/problem+json; charset=utf-8','APPLICATION/JSON','application/vnd.json','application/json-stream','application/json_stream','application/notjson','invalid','application/json extra','application/json+zip','application/zip+json-stream','x'.repeat(128)+'/json','']) { const c=create();output.mime.push({input,expected:run(c,`requireModule('utils/fetch-utils').isJSON(${JSON.stringify(input)})`)}); }
 const err=create();await launch(err,'coins','',{title:'SYNTHETIC_ERROR',detail:'Owned error detail'});assert.equal(run(err,'failures[0]'),'Owned error detail');assert.equal(run(err,'requests.length'),1);await flush(err);assert.equal(run(err,'requests.length'),1);
 assert.equal(run(err,'failures[0]'), 'Owned error detail');
 // Closed import, no process/file system/network access, and no dynamic code.
 for(const code of ['requireModule("node:fs")','requireModule("node:https")','requireModule("node:child_process")','Function("return process")()']) assert.throws(()=>run(create(),code));
 assert.equal(run(create(),'typeof process'),'undefined');
 const target=path.join(root,'tests/source-oracle.json');fs.writeFileSync(target,JSON.stringify(output,null,2)+'\n');
 console.log(`Source oracle: ${output.views.length} full views, ${output.numeric.length} argument cases, ${output.currency.length} currency cases, both command cache/request lifecycles, empty/error paths, 5 escape checks passed.`);
})().catch(e=>{console.error(e);process.exitCode=1;});
