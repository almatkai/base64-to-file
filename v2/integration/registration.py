#!/usr/bin/env python3
"""Read a supplied host tree and print an additive patch. Never writes to host."""
import argparse,difflib,json,pathlib,re,sys
parser=argparse.ArgumentParser();parser.add_argument('host',type=pathlib.Path);args=parser.parse_args()
root=args.host;here=pathlib.Path(__file__).resolve().parent.parent
manifest=json.loads((here/'extension.v2.json').read_text())
changes={}
def update(path,change):
    old=(root/path).read_text();new=change(old)
    if new==old:raise ValueError('No change: '+path)
    changes[path]=(old,new)
def once(text,old,new):
    if text.count(old)!=1:raise ValueError('Expected exactly one anchor: '+repr(old))
    return text.replace(old,new,1)
try:
    mod='src-tauri/src/native_core/extensions/mod.rs'
    if 'v2_base64_to_file' in (root/mod).read_text():raise ValueError('Adapter already registered; review existing changes')
    update(mod,lambda s:once(s,'mod native;','mod native;\n#[path = "../../../native-extensions/base64-to-file/adapter.rs"]\nmod v2_base64_to_file;'))
    runtime='src-tauri/src/native_core/extensions/v2_runtime.rs'
    def dispatch(s):
        # Preserve every current dispatch branch, including previous sequential ports.
        s=once(s,'match value {','match value {\n        "base64-to-file" => Some(value),')
        marker='pub(super) async fn handle('
        if s.count(marker)!=1:raise ValueError('Cannot locate handle')
        before,after=s.split(marker)
        before=once(before,'match slug {','match slug {\n        "base64-to-file" => super::v2_base64_to_file::commands(),')
        after=once(after,'match slug {','match slug {\n        "base64-to-file" => super::v2_base64_to_file::handle(ctx, command, operation, payload, state).await,')
        return before+marker+after
    update(runtime,dispatch)
    cat='src-tauri/src/native_core/extensions/v2_catalog.json'
    current=json.loads((root/cat).read_text())
    if any(v.get('slug')=='base64-to-file' for v in current):raise ValueError('Catalog entry already exists')
    def catalog(s):
        end=s.rfind(']');return s[:end].rstrip()+',\n'+json.dumps(manifest,indent=2).replace('\n','\n  ').join(['  ',''])+'\n'+s[end:]
    update(cat,catalog)
    count='src-tauri/src/native_core/extensions/v2_catalog.rs'
    def counts(s):
        matches=list(re.finditer(r'assert_eq!\(items\.len\(\),\s*(\d+)\);',s))
        if len(matches)!=1:raise ValueError('Catalog count anchor changed; reconcile manually')
        m=matches[0]
        if int(m[1])!=len(current):raise ValueError('Existing catalog count differs; reconcile before port')
        return s[:m.start()]+f'assert_eq!(items.len(), {len(current)+1});'+s[m.end():]
    update(count,counts)
    cargo='src-tauri/Cargo.toml'
    def dependency(s):
        if re.search(r'^cap-std\s*=',s,re.M):return s # caller handles existing dependency
        return once(s,'[dependencies]','[dependencies]\ncap-std = "=3.4.5"')
    if not re.search(r'^cap-std\s*=',(root/cargo).read_text(),re.M):update(cargo,dependency)
    for path,(old,new) in changes.items():
        sys.stdout.writelines(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
except (ValueError,OSError) as e:
    print('Refusing to generate registration patch: '+str(e),file=sys.stderr);sys.exit(1)
