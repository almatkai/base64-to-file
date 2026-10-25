#!/usr/bin/env python3
"""Inventory fixtures and exact source references without using the host checkout."""
import hashlib,json,pathlib,re
r=pathlib.Path(__file__).resolve().parent.parent
own='fdf73d21288398f754524993e9d65b012c90fe08';beta='fd300e276356587726566026f072563eb9c34cb1';upstream='6734d85992290066fd31a7f5bee3cca3d3545a1a'
def entry(p):
 b=p.read_bytes();return {'path':p.relative_to(r).as_posix(),'size':len(b),'sha256':hashlib.sha256(b).hexdigest()}
files=[]
for p in sorted((r/'fixtures').rglob('*')):
 if not p.is_file():continue
 e=entry(p);rel=p.relative_to(r/'fixtures').as_posix()
 if rel.startswith('v1/'):
  e['source']='https://raw.githubusercontent.com/almatkai/base64-to-file/'+own+'/'+rel[3:]
 elif rel.startswith('beta/'):
  e['source']='Tezbar git object '+beta+':'+rel[5:]
 elif rel.startswith('upstream/'):
  name=p.name
  paths={'package.json':'extensions/base64-to-file/package.json','command-icon.png':'extensions/base64-to-file/assets/command-icon.png','LICENSE':'LICENSE'}
  e['source']='https://raw.githubusercontent.com/raycast/extensions/'+upstream+'/'+paths[name] if name in paths else 'Generated HTTP/license lookup evidence'
 elif rel.startswith('utils/'):
  e['source']='Own V1 locked npm @raycast/utils '+json.loads((r/'fixtures/utils/package.json').read_text())['version']
  if p.name in ['useForm.js','useLatest.js']:
   full=(r/'fixtures/utils/main.js').read_bytes();excerpt=p.read_bytes();start=full.find(excerpt);assert start>=0;e['byteRange']={'startInclusive':start,'endExclusive':start+len(excerpt)};e['parent']='fixtures/utils/main.js'
 elif rel.startswith('typescript/'):
  e['source']='Own V1 locked npm typescript '+json.loads((r/'fixtures/typescript/package.json').read_text())['version']
 elif rel.startswith('schema-validator/'):
  e['source']='Ajv 8.17.1 validation-only npm install; exact resolution in fixtures/schema-validator/package-lock.json'
 elif p.name.startswith('offline-support'):
  e['source']='Deterministic support archive/member inventory; registry checksums in included Cargo locks and cargo-checksum metadata'
 elif 'transcript' in p.name:e['source']='Generated successful validation transcript; clone path normalized to <V2_ROOT>'
 elif 'xerj' in p.name:e['source']='Generated bounded final XERJ indexing attempt; frozen-schema failure recorded'
 else:e['source']='Generated executable V1 oracle / concrete adapter disposable-test capture' 
 files.append(e)
lock=json.loads((r/'fixtures/v1/package-lock.json').read_text())
libs=[]
for name in ['@raycast/utils','typescript']:
 info=lock['packages']['node_modules/'+name];libs.append({'package':name,'version':info['version'],'resolved':info.get('resolved'),'integrity':info.get('integrity'),'license':info.get('license')})
licenses=[e for e in files if re.search(r'(LICENSE|LICENCE|COPYING|NOTICE|license|ThirdPartyNotice)',pathlib.Path(e['path']).name)]
result={'ownV1':{'repository':'https://github.com/almatkai/base64-to-file','commit':own},'beta':{'commit':beta,'access':'git show only; no host worktree files copied'},'original':{'repository':'https://github.com/raycast/extensions','commit':upstream,'author':'forrestzh'},'lockedLibraries':libs,'files':files,'licenseFiles':licenses,'registrySupportMembers':'fixtures/offline-support.inventory.json','limits':['Original extension-specific LICENSE URL returned 404; repository-root license retained','@raycast/utils package declares MIT but contains no LICENSE file','No separate icon-specific grant evidenced','No real desktop/Windows execution QA performed']}
(r/'PROVENANCE.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
print('Inventoried',len(files),'fixture files including',len(licenses),'license/notice files')
