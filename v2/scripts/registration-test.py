#!/usr/bin/env python3
import json,pathlib,shutil,subprocess,tempfile
root=pathlib.Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='base64-registration-') as tmp:
 dest=pathlib.Path(tmp);shutil.copytree(root/'fixtures/beta',dest,dirs_exist_ok=True)
 patch=subprocess.check_output(['python3',str(root/'integration/registration.py'),str(dest)])
 assert patch==(root/'integration/pinned-beta-registration.patch').read_bytes()
 subprocess.run(['git','apply','--check','--unsafe-paths','-'],input=patch,cwd=dest,check=True)
 subprocess.run(['git','apply','--unsafe-paths','-'],input=patch,cwd=dest,check=True)
 catalog=json.loads((dest/'src-tauri/src/native_core/extensions/v2_catalog.json').read_text());assert len(catalog)==12
 assert catalog[-1]==json.loads((root/'extension.v2.json').read_text())
 # Simulate a preceding sequential port. Generate against that tree and verify it
 # retains the additional catalog entry, count, module and runtime branches.
 extra=dict(catalog[-1]);extra['slug']='earlier-port';extra['id']='com.tezbar.native.earlier-port'
 original=json.loads((root/'fixtures/beta/src-tauri/src/native_core/extensions/v2_catalog.json').read_text());original.append(extra)
 shutil.copytree(root/'fixtures/beta',dest,dirs_exist_ok=True)
 (dest/'src-tauri/src/native_core/extensions/v2_catalog.json').write_text(json.dumps(original,indent=2)+'\n')
 count=dest/'src-tauri/src/native_core/extensions/v2_catalog.rs';count.write_text(count.read_text().replace('items.len(), 11','items.len(), 12'))
 runtime=dest/'src-tauri/src/native_core/extensions/v2_runtime.rs';runtime.write_text(runtime.read_text().replace('match value {','match value {\n        "earlier-port" => Some(value),'))
 mod=dest/'src-tauri/src/native_core/extensions/mod.rs';mod.write_text(mod.read_text()+'\nmod v2_earlier_port;\n')
 patch=subprocess.check_output(['python3',str(root/'integration/registration.py'),str(dest)])
 subprocess.run(['git','apply','--check','--unsafe-paths','-'],input=patch,cwd=dest,check=True);subprocess.run(['git','apply','--unsafe-paths','-'],input=patch,cwd=dest,check=True)
 cat=json.loads((dest/'src-tauri/src/native_core/extensions/v2_catalog.json').read_text());assert len(cat)==13 and cat[-2]==extra
 assert 'items.len(), 13' in count.read_text();assert '"earlier-port" => Some(value)' in runtime.read_text();assert 'mod v2_earlier_port;' in mod.read_text()
print('Registration dry run on disposable pinned host and preceding-port fixture: PASS (real host untouched)')
