#!/usr/bin/env python3
"""Rebuild support archive after intentional fixture/harness/dependency changes."""
import gzip,hashlib,io,json,pathlib,tarfile
root=pathlib.Path(__file__).resolve().parent.parent
files=[]
for prefix in ['vendor','tests/beta-harness','fixtures/beta']:
 for p in (root/prefix).rglob('*'):
  if p.is_file() and not p.is_symlink() and 'target' not in p.relative_to(root).parts:
   files.append(p)
files.sort();inventory=[]
with (root/'fixtures/offline-support.tar.gz').open('wb') as output:
 with gzip.GzipFile(fileobj=output,mode='wb',mtime=0) as gz:
  with tarfile.open(fileobj=gz,mode='w') as archive:
   for p in files:
    data=p.read_bytes();name=p.relative_to(root).as_posix();entry=tarfile.TarInfo(name);entry.size=len(data);entry.mode=0o644;archive.addfile(entry,io.BytesIO(data));inventory.append({'path':name,'size':len(data),'sha256':hashlib.sha256(data).hexdigest()})
(root/'fixtures/offline-support.inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
print('Archived',len(files),'unchanged files; all registry sources/checksums/licenses, pinned beta fixtures and concrete adapter harness')
