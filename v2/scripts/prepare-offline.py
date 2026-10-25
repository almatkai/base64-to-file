#!/usr/bin/env python3
"""Expand the packaged support archive locally, verifying every member first.
Cargo omits nested Cargo.toml packages even with explicit include globs. This
archive retains those required sources without relying on the original checkout.
"""
import hashlib,json,pathlib,tarfile
root=pathlib.Path(__file__).resolve().parent.parent
expected={e['path']:e for e in json.loads((root/'fixtures/offline-support.inventory.json').read_text())}
with tarfile.open(root/'fixtures/offline-support.tar.gz','r:gz') as archive:
 members=archive.getmembers()
 if len(members)!=len(expected):raise SystemExit('Offline support member count mismatch')
 verified=[]
 for member in members:
  path=pathlib.PurePosixPath(member.name)
  if path.is_absolute() or '..' in path.parts or not member.isfile() or member.name not in expected:raise SystemExit('Unsafe support archive member')
  data=archive.extractfile(member).read();e=expected[member.name]
  if len(data)!=e['size'] or hashlib.sha256(data).hexdigest()!=e['sha256']:raise SystemExit('Support archive hash mismatch: '+member.name)
  dest=root.joinpath(*path.parts)
  if dest.exists() and dest.read_bytes()!=data:raise SystemExit('Refusing to overwrite changed source: '+member.name)
  verified.append((dest,data))
 for dest,data in verified:
  if not dest.exists():dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
print('Offline support verified; local vendor and beta harness ready')
