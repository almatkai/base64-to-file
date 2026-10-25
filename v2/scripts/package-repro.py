#!/usr/bin/env python3
"""Package and run extracted sources offline with an EMPTY Cargo home.
Leaves disposable build evidence under test-output/. Never accesses a host tree.
"""
import os,pathlib,subprocess,tarfile,tempfile
root=pathlib.Path(__file__).resolve().parent.parent
subprocess.run(['cargo','package','--allow-dirty','--no-verify','--locked','--offline'],cwd=root,check=True)
out=root/'test-output';out.mkdir(exist_ok=True);work=pathlib.Path(tempfile.mkdtemp(prefix='package-repro-',dir=out))
archive=root/'target/package/tezbar-base64-to-file-0.1.0.crate'
with tarfile.open(archive) as tar:
 for m in tar.getmembers():
  p=pathlib.PurePosixPath(m.name)
  if p.is_absolute() or '..'in p.parts or not(m.isfile()or m.isdir()):raise SystemExit('Unsafe package archive')
 tar.extractall(work)
pkg=work/'tezbar-base64-to-file-0.1.0';env=os.environ.copy();env['CARGO_HOME']=str(work/'empty-cargo-home');env['CARGO_TARGET_DIR']=str(work/'target')
commands=[['python3','scripts/prepare-offline.py'],['node','scripts/schema.cjs'],['node','scripts/v1-oracle.cjs',str(work/'oracle.json')],['node','scripts/renderer.cjs'],['python3','scripts/registration-test.py']]
for tc in ['stable','1.85.0']:
 commands += [['cargo','+'+tc,'test','--locked','--offline'],['cargo','+'+tc,'test','--manifest-path','tests/beta-harness/Cargo.toml','--locked','--offline']]
for command in commands:
 print('RUN',' '.join(command),flush=True);subprocess.run(command,cwd=pkg,env=env,check=True)
print('PASS: extracted Cargo package + empty Cargo home offline stable/1.85 core and unchanged-beta harness tests',flush=True)
print('Evidence retained:',work,flush=True)
