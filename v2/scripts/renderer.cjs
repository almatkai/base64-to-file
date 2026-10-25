// Actual pinned beta parser/reconciliation and surface handlers; React/UI only stubbed.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),ts=require(path.join(root,'fixtures/typescript/typescript.js'));
const base=path.join(root,'fixtures/beta/src/renderer/src/raycast-api');
let states=[],index=0;
const react={forwardRef:fn=>fn,useMemo:fn=>fn(),useCallback:fn=>fn,useRef:v=>({current:v}),useEffect(){},useImperativeHandle(){},createContext:()=>({Provider:'Provider'}),useContext:()=>({actions:[]}),useState(v){let i=index++;if(!(i in states))states[i]=typeof v==='function'?v():v;return[states[i],v=>states[i]=typeof v==='function'?v(states[i]):v]}};
const cache={};
function load(name){
 if(cache[name])return cache[name];let extra=name==='form-runtime'?'\nexport {collectFormItems};':name==='list-runtime'?'\nexport {parseListRows};':'';
 const src=fs.readFileSync(path.join(base,name+'.tsx'),'utf8')+extra;
 const code=ts.transpileModule(src,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022}}).outputText;
 const jsx=(type,props)=>({type,props});const ctx={exports:{},console,window:{addEventListener(){},removeEventListener(){}},HTMLTextAreaElement:class{}};
 ctx.require=id=>{
  if(id==='react')return {...react,default:react};if(id==='react/jsx-runtime')return {jsx,jsxs:jsx};
  if(id.startsWith('./')&&['detail-runtime','form-runtime','list-runtime','action-runtime','action-runtime-overlay','action-runtime-registry'].includes(id.slice(2)))return load(id.slice(2));
  if(id==='./hooks')return {setRuntimeContext(){},NavigationContext:{Provider:'Navigation'}};
  if(id==='./grid-runtime')return {GridRuntime:'GridRuntime'};
  return new Proxy({}, {get:(_,key)=>key==='cx'?((...x)=>x.join(' ')):String(key)});
 };
 vm.createContext(ctx);vm.runInContext(code,ctx);cache[name]=ctx.exports;return ctx.exports;
}
function find(node,pred){if(!node)return; if(Array.isArray(node)){for(const n of node){let r=find(n,pred);if(r)return r}return}if(typeof node!=='object')return;if(pred(node))return node;return find(node.props?.children,pred)}
const form=load('form-runtime'),detail=load('detail-runtime'),list=load('list-runtime'),surface=load('index');
// Adapter samples are exported by the real Rust adapter test, not handwritten UI.
const samplePath=process.argv[2]||path.join(root,'fixtures/adapter-views.json');const views=JSON.parse(fs.readFileSync(samplePath));
assert(views.validation.message.startsWith('Error:'));
assert(views.revealFailure.message.includes('failed or was not confirmed'));
for(const view of Object.values(views))assert.equal(view.message,view.root.children[0].props.text);
const first=views.initial,fields=form.collectFormItems(first.root).filter(x=>x.type==='field').map(x=>x.field);
assert.deepEqual(Array.from(fields,f=>f.id),['base64','filePath']);let values=form.reconcileFormValues(fields);assert.equal(values.base64,'');assert.match(values.filePath,/Downloads[/\\]out.pdf$/);
const draft={base64:'newer typed draft',filePath:'/newer/path.bin'};
for(const view of [views.validation,views.saved,views.revealFailure,views.refreshed]){
 const next=form.collectFormItems(view.root).filter(x=>x.type==='field').map(x=>x.field);assert.equal(next[0].value,undefined);assert.equal(next[0].actionId,undefined);
 assert.deepEqual(JSON.parse(JSON.stringify(form.reconcileFormValues(next,draft,fields))),draft);
}
let calls=[];states=[];index=0;
const tree=surface.ExtensionRuntimeSurface({sessionId:'fixture',title:first.title,extensionId:'com.tezbar.native.base64-to-file',commandName:'index',root:first.root,actions:first.actions,onBack(){},onSearchTextChanged(){},onLoadMore(){},onInvokeAction:(...args)=>calls.push(args)});
const formNode=find(tree,n=>n.type===form.FormRuntime);assert(formNode);formNode.props.onSubmitForm(draft);assert.deepEqual(calls.pop(),[first.actions[0].id,draft]);formNode.props.onChangeField('field-action','value');assert.deepEqual(JSON.parse(JSON.stringify(calls.pop())),['field-action',{value:'value'}]);
assert.equal(first.actions[0].kind,'submit-form');for(const view of [views.saved,views.revealFailure]){assert.equal(view.actions[0].kind,'action');assert.equal(view.actions[1].title,'Convert Another File');}
for(const v of Object.values(views)){assert(v.actions.every(a=>!('shortcut'in a)));assert(!JSON.stringify(v.root).includes('tooltip'));}
// Run the actual Form component submit callback and textarea preservation too.
states=[];index=0;const rendered=form.FormRuntime({...formNode.props,root:first.root,title:first.title,onSubmitForm:v=>calls.push(v)},null);const submit=find(rendered,n=>n.type==='form');submit.props.onSubmit({preventDefault(){}});assert.equal(calls.pop().base64,'');
assert.deepEqual(JSON.parse(JSON.stringify(list.parseListRows({type:'List',children:[]}))),[]);
assert.equal(detail.parseNativeColorWheel(undefined),null);
states=[];index=0;const detailTree=detail.DetailRuntime({root:{type:'Detail',props:{markdown:'File saved.'}},title:'Saved',onBack(){},onRunPrimaryAction(){},onOpenActions(){}});assert(detailTree);
console.log('Pinned Form parser/reconciliation, full submit maps, field callback value shape, actual surface handlers, Detail/List parser smoke and action ordering: PASS');
