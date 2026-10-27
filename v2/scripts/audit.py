#!/usr/bin/env python3
"""Read-only Git/provenance visibility audit. Never stages or writes to Git."""
import hashlib,json,pathlib,subprocess
v2=pathlib.Path(__file__).resolve().parent.parent;root=v2.parent
p=json.loads((v2/'PROVENANCE.json').read_text())
for e in p['files']:
 data=(v2/e['path']).read_bytes();assert len(data)==e['size'] and hashlib.sha256(data).hexdigest()==e['sha256'],e['path']
# Packaged/extracted validation has no Git repository requirement.
if subprocess.run(['git','rev-parse','--show-toplevel'],cwd=root,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0:
 def git(*args):return subprocess.check_output(['git',*args],cwd=root)
 assert git('branch','--show-current').decode().strip()=='feat/native-v2-rust'
 assert git('rev-parse','HEAD').decode().strip()=='fdf73d21288398f754524993e9d65b012c90fe08'
 assert not git('diff','--cached','--name-only'),'Index must be clean'
 for name in git('ls-tree','-r','--name-only','HEAD').decode().splitlines():
  old=git('show','HEAD:'+name);current=(root/name).read_bytes()
  if name=='README.md':assert current==old+b'\nRust Native V2 port and integration notes: [v2/README.md](v2/README.md).\n'
  else:assert old==current,'V1 changed: '+name
 visible=set(git('ls-files','--others','--exclude-standard','v2').decode().splitlines())|set(git('ls-files','v2').decode().splitlines())
 for prefix in ['src','scripts','fixtures','integration','assets','tests']:
  for file in (v2/prefix).rglob('*'):
   if file.is_file() and 'target' not in file.relative_to(v2).parts:assert file.relative_to(root).as_posix() in visible,'Git-invisible required fixture: '+str(file)
 print('All V1 tracked bytes preserved except exact README discovery link; index clean; required V2 fixtures Git-visible')
print('Provenance hashes: PASS')
