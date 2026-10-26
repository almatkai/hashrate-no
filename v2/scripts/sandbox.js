'use strict';
const factories=Object.create(null),modules=Object.create(null),cacheBacking=new Map();
let fixtureData=[],requests=[],failures=[],engine;
const console={log(){},warn(){},error(){}};
// Pure VM producers: none expose a host JS Function constructor or real I/O.
class Cache {
 constructor(options={}){this.prefix=options.namespace??'';this.subscribe=()=>()=>{};}
 get(key){return cacheBacking.get(this.prefix+':'+key)}
 set(key,value){cacheBacking.set(this.prefix+':'+key,value);engine.dirty=true;}
 remove(key){cacheBacking.delete(this.prefix+':'+key);engine.dirty=true;}
}
class FixedDate extends Date { constructor(...args){super(...(args.length?args:['2026-01-02T12:00:00Z']));} static now(){return 1767355200000;} }
globalThis.Date=FixedDate;
class AbortController { constructor(){this.signal={aborted:false};} abort(){this.signal.aborted=true;} }
const AbortSignal={any(a){return a[0]}};
class URL {}
class Request {}
class URLSearchParams {
 constructor(values){this.values=values;}
 toString(){const enc=s=>encodeURIComponent(String(s)).replace(/[!'()~]/g,c=>'%'+c.charCodeAt(0).toString(16).toUpperCase()).replace(/%20/g,'+');return Object.entries(this.values).map(([k,v])=>enc(k)+'='+enc(v)).join('&');}
}
async function fetch(url,options){
 if(!/^https:\/\/api\.hashrate\.no\/v1\/(coins|gpuEstimates)\?apiKey=ORACLE-SYNTHETIC-KEY(?:&powerCost=[^&]*)?$/.test(url))throw Error('network escape denied');
 requests.push({url,method:'GET'});
 return {ok:true,status:200,headers:{get(k){return k==='content-type'?'application/json':null;}},json:async()=>fixtureData,text:async()=>JSON.stringify(fixtureData)};
}
const jsx=(type,props,key)=>({type,props:props??{},key});
const token=name=>Object.assign(()=>{},{toString:()=>name});
const List=token('List');List.Section=token('List.Section');List.Item=token('List.Item');List.Item.Detail=token('List.Item.Detail');List.Item.Detail.Metadata=token('List.Item.Detail.Metadata');List.Item.Detail.Metadata.Label=token('List.Item.Detail.Metadata.Label');List.Item.Detail.Metadata.Separator=token('List.Item.Detail.Metadata.Separator');
const Action=token('Action');const api={List,Action,ActionPanel:token('ActionPanel'),Icon:new Proxy({}, {get:(_,p)=>'Icon.'+p}),getPreferenceValues:()=>({api_key:'ORACLE-SYNTHETIC-KEY'}),Cache,popToRoot:async()=>{},environment:{launchType:'user',commandMode:'view'},LaunchType:{Background:'background'}};
const equal=(a,b)=>a===b||(Array.isArray(a)&&Array.isArray(b)&&a.length===b.length&&a.every((x,i)=>equal(x,b[i])))||(a&&b&&typeof a==='object'&&typeof b==='object'&&!Array.isArray(a)&&!Array.isArray(b)&&Object.keys(a).length===Object.keys(b).length&&Object.keys(a).every(k=>equal(a[k],b[k])));
const react={
 useState(initial){const i=engine.cursor++;if(!(i in engine.slots))engine.slots[i]=typeof initial==='function'?initial():initial; if(!engine.setters[i])engine.setters[i]=next=>{const value=typeof next==='function'?next(engine.slots[i]):next;if(!Object.is(value,engine.slots[i])){engine.slots[i]=value;engine.dirty=true;}};return [engine.slots[i],engine.setters[i]];},
 useRef(initial){const i=engine.cursor++;return engine.slots[i]??(engine.slots[i]={current:initial});},
 useEffect(fn,deps){const i=engine.cursor++;const prev=engine.slots[i];if(!prev||!deps||!equal(prev.deps,deps)){if(prev?.cleanup)prev.cleanup();engine.slots[i]={deps};engine.effects.push(()=>{engine.slots[i].cleanup=fn();});}},
 useMemo(fn,deps){const i=engine.cursor++;const prev=engine.slots[i];if(prev&&equal(prev.deps,deps))return prev.value;const value=fn();engine.slots[i]={deps,value};return value;},
 useCallback(fn,deps){return react.useMemo(()=>fn,deps);},
 useSyncExternalStore(_sub,get){engine.cursor++;return get();},
};
engine={slots:[],setters:[],effects:[],cursor:0,dirty:false,tree:null,
 mount(fn){for(const s of this.slots)if(s?.cleanup)s.cleanup();this.slots=[];this.setters=[];this.fn=fn;this.render();},
 render(){this.cursor=0;this.dirty=false;this.effects=[];this.tree=this.fn();for(const f of this.effects)f();},
};
// SHA-1 producer for the locked helper. Its input/output is entirely inside VM.
function sha1(text){
 const bytes=[];for(const c of unescape(encodeURIComponent(text)))bytes.push(c.charCodeAt(0));const len=bytes.length;bytes.push(128);while(bytes.length%64!==56)bytes.push(0);for(let i=7;i>=0;i--)bytes.push(i<4?((len*8)>>>i*8)&255:0);
 let h=[0x67452301,0xefcdab89,0x98badcfe,0x10325476,0xc3d2e1f0];const rol=(n,s)=>(n<<s)|(n>>>(32-s));
 for(let off=0;off<bytes.length;off+=64){const w=[];for(let i=0;i<16;i++)w[i]=(bytes[off+4*i]<<24)|(bytes[off+4*i+1]<<16)|(bytes[off+4*i+2]<<8)|bytes[off+4*i+3];for(let i=16;i<80;i++)w[i]=rol(w[i-3]^w[i-8]^w[i-14]^w[i-16],1);let[a,b,c,d,e]=h;for(let i=0;i<80;i++){const f=i<20?(b&c)|(~b&d):i<40?b^c^d:i<60?(b&c)|(b&d)|(c&d):b^c^d;const k=i<20?0x5a827999:i<40?0x6ed9eba1:i<60?0x8f1bbcdc:0xca62c1d6;const t=(rol(a,5)+f+e+k+w[i])|0;e=d;d=c;c=rol(b,30);b=a;a=t;}h=h.map((n,i)=>(n+[a,b,c,d,e][i])|0);}
 return h.map(n=>(n>>>0).toString(16).padStart(8,'0')).join('');
}
const crypto={createHash(name){if(name!=='sha1')throw Error('unsupported crypto');let data='';return {update(value){data+=value;return this;},write(value){data+=value;return this;},digest(format){if(format!=='hex')throw Error('unsupported encoding');return sha1(data);}};}};
const Buffer={isBuffer:()=>false};
function resolve(parent,spec){if(!spec.startsWith('.'))return spec;const bits=parent.split('/');bits.pop();for(const x of spec.split('/')){if(x==='..')bits.pop();else if(x!=='.')bits.push(x);}return bits.join('/');}
function requireModule(name){
 if(name==='react')return react;
 if(name==='react/jsx-runtime')return {jsx,jsxs:jsx,Fragment:'Fragment'};
 if(name==='@raycast/api')return api;
 if(name==='@raycast/utils')return {...requireModule('utils/useCachedState'),...requireModule('utils/useFetch'),showFailureToast:async message=>failures.push(message)};
 if(name==='utils/showFailureToast')return {showFailureToast:async message=>failures.push(message)};
 if(name==='node:crypto')return {default:crypto,__esModule:true};
 if(name==='node:util')return {types:{isNativeError:v=>v instanceof Error}};
 if(name==='fast-deep-equal')return {default:equal,__esModule:true};
 if(modules[name])return modules[name].exports;
 if(!factories[name])throw Error('import escape denied: '+name);
 const module={exports:{}};modules[name]=module;factories[name](module.exports,spec=>requireModule(resolve(name,spec)),module);return module.exports;
}
function descendants(node,predicate){let out=[];if(!node||typeof node!=='object')return out;if(predicate(node))out.push(node);for(const child of [node.props?.children,node.props?.actions,node.props?.metadata].flat(Infinity))out.push(...descendants(child,predicate));return out;}
function treeActions(){return descendants(engine.tree,n=>n.type===Action);}
function project(tree){return descendants(tree,n=>n.type===List.Item).map(n=>({title:n.props.title??'Untitled',subtitle:n.props.subtitle??null,accessories:n.props.accessories??[],metadata:descendants(n.props.detail,x=>x.type===List.Item.Detail.Metadata.Label||x.type===List.Item.Detail.Metadata.Separator).map(x=>x.type===List.Item.Detail.Metadata.Separator?{separator:true}:{title:x.props.title,text:x.props.text??null})}));}
