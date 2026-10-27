// Executes immutable own TSX + the locked utils useForm implementation.
// Only React/UI/OS (storage, homedir, streams, reveal, toast) are recording stubs.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {EventEmitter}=require('node:events');
const root=path.resolve(__dirname,'..');
const ts=require(process.env.TEZBAR_TYPESCRIPT || path.join(root,'fixtures','typescript','typescript.js'));
const source=fs.readFileSync(path.join(root,'fixtures/v1/src/index.tsx'),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022}}).outputText;
function fixture(stored){
 let slots=[],index=0,effects=[],events=[],dirty=false;
 const hooks={
  useState(init){let i=index++;if(!(i in slots))slots[i]=typeof init==='function'?init():init;return [slots[i],v=>{slots[i]=typeof v==='function'?v(slots[i]):v;dirty=true}]},
  useRef(v){let i=index++;return slots[i] ||= {current:v}},
  useCallback(fn){return fn},useMemo(fn){return fn()},
  useEffect(fn,deps){let i=index++;if(!slots[i] || !deps || deps.some((v,j)=>!Object.is(v,slots[i][j]))){slots[i]=deps;effects.push(fn)}}
 };
 const ctx={exports:{},Buffer,Blob,Uint8Array,atob,console:{log(){}},Date};
 ctx.$8zHUo$react=hooks;
 vm.createContext(ctx);
 vm.runInContext(fs.readFileSync(path.join(root,'fixtures/utils/useLatest.js'),'utf8')+'\n'+fs.readFileSync(path.join(root,'fixtures/utils/useForm.js'),'utf8')+'\nthis.actualUseForm=$c950f3189e9100ee$export$87c0cf8eb5a167e0;this.FormValidation=$c950f3189e9100ee$export$cd58ffd7e3880e66;',ctx);
 const component=(type)=>type; const Form=Object.assign('Form',{Description:'Form.Description',TextArea:'Form.TextArea',TextField:'Form.TextField'});
 const api={Form,ActionPanel:'ActionPanel',Action:{SubmitForm:'Action.SubmitForm'},Toast:{Style:{Success:'success',Failure:'failure'}},
 LocalStorage:{async getItem(k){events.push(['get',k]);return stored},async setItem(k,v){events.push(['set',k,v])}},
 showToast(v){events.push(['toast',v])},async showInFinder(p){events.push(['reveal',p]);if(p==='reveal-error')throw Error('reveal failed')}
 };
 ctx.require=(id)=>{
  if(id==='react')return hooks;
  if(id==='react/jsx-runtime')return {jsx:(type,props)=>({type,props}),jsxs:(type,props)=>({type,props})};
  if(id==='@raycast/api')return api;
  if(id==='@raycast/utils')return {useForm:ctx.actualUseForm,FormValidation:ctx.FormValidation};
  if(id==='os')return {homedir:()=>'/oracle-home'};
  if(id==='fs')return {createWriteStream(p){events.push(['createWriteStream',p]);const stream=new EventEmitter();stream.write=b=>events.push(['write',Buffer.from(b).toString('hex')]);stream.end=()=>{events.push(['end']);queueMicrotask(()=>{events.push([p==='write-error'?'error':'finish']);stream.emit(p==='write-error'?'error':'finish',p==='write-error'?Error('write failed'):undefined)})};return stream}};
  throw Error('Unexpected import '+id);
 };
 vm.runInContext(js,ctx);
 function render(){index=0;dirty=false;return ctx.exports.default()}
 async function settle(){for(let i=0;i<8;i++){let jobs=effects;effects=[];jobs.forEach(f=>f());await Promise.resolve();if(dirty)render();if(!effects.length&&!dirty)break}}
 const tree=render();
 return {events,tree,settle,async submit(values){let before=events.length;let result=await tree.props.actions.props.children.props.onSubmit(values);return {result,events:events.slice(before)}}};
}
(async()=>{
 const cases=['Zg==','Zg','Zg=','Zh==','Zm8=','Zm9=','Zm9v','Zg===','A','AA','AAA','AAAA','====',' Z\tg\n=\r=\f','Zg\v==','Zg\u00a0==','-_8=','+/8=','AP+A','😀','é','data:application/pdf;base64,Zg==','',' ','\t\n\r\f'];
 let outputs=[];
 for(const input of cases){const f=fixture();const r=await f.submit({base64:input,filePath:'/selected/output.bin'});outputs.push({input,result:r.result,events:r.events});}
 const e=fixture();assert.equal((await e.submit({base64:'',filePath:'/x'})).result,false);assert.equal((await e.submit({base64:'AA==',filePath:''})).result,false);
 const p=fixture();assert.equal(p.tree.props.children[2].props.defaultValue,'/oracle-home/Downloads/out.pdf');assert.equal(p.tree.props.children[1].props.value,undefined);assert.equal(p.tree.props.children[1].props.onChange,undefined);
 for(const filepath of ['relative.bin','~/out.pdf',' /tmp/space ','write-error','reveal-error']){const f=fixture();outputs.push({filePath:filepath,...await f.submit({base64:'AP+A',filePath:filepath})});}
 let storage=[];
 for(const stored of [undefined,'{"base64":"secret draft","filePath":"/old"}','{broken','null','17']){
  const f=fixture(stored);await f.settle();const before=JSON.stringify(f.events);await f.submit({base64:'Zg==',filePath:'/new'});storage.push({stored:stored??null,loadEvents:JSON.parse(before),allEvents:f.events});
  const sets=f.events.filter(e=>e[0]==='set');assert(sets.every(e=>!e[2].includes('/new')));
 }
 const success=outputs[0].events.map(e=>e[0]);assert.deepEqual(success,['createWriteStream','write','end','finish','toast','reveal']);
 assert.equal(outputs.find(x=>x.input==='Zh==').events.find(e=>e[0]==='write')[1],'66');
 assert.equal(outputs.find(x=>x.input==='AP+A').events.find(e=>e[0]==='write')[1],'00ff80');
 assert.equal(outputs.find(x=>x.input===' ').events.find(e=>e[0]==='write')[1],'');
 const result={node:process.version,source:'fixtures/v1/src/index.tsx',utils:'fixtures/utils/main.js (unchanged useForm/useLatest excerpts)',outputs,storage};
 const out=process.argv[2];if(out)fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n');else console.log(JSON.stringify(result,null,2));
})().catch(e=>{console.error(e);process.exitCode=1});
