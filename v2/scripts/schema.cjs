const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..');
const Ajv=require(path.join(root,'fixtures/schema-validator/node_modules/ajv/dist/2020.js'));
const schema=JSON.parse(fs.readFileSync(path.join(root,'fixtures/beta/schemas/extension-v2.schema.json')));
const ajv=new Ajv({strict:true,allErrors:true});const validate=ajv.compile(schema);
const manifest=JSON.parse(fs.readFileSync(path.join(root,'extension.v2.json')));
assert(validate(manifest),JSON.stringify(validate.errors));
const negative=[m=>m.author='forrestzh',m=>m.runtime='javascript',m=>m.schemaVersion=1,m=>m.id='raycast.base64-to-file',m=>m.icon='https://wrong/icon.png',m=>m.replaces=[],m=>m.commands=[],m=>m.commands[0].platforms=['macos'],m=>m.verification[0].state='tested',m=>delete m.icon,m=>m.unexpected=true];
for(const change of negative){const m=structuredClone(manifest);change(m);assert.equal(validate(m),false);}
assert.deepEqual(manifest.verification.map(x=>[x.platform,x.state]),[['macOS','not-run'],['Windows','not-run']]);
for(const filename of ['assets/command-icon.png','fixtures/upstream/command-icon.png']){const b=fs.readFileSync(path.join(root,filename));assert.equal(b.length,13421);assert.equal(crypto.createHash('sha256').update(b).digest('hex'),'e1a8868a61378242995331e9d156f8c53f70adb62fe04cc7cf831bcec757eb27');}
console.log('Pinned full draft-2020 schema: manifest + 11 negative cases; exact original icon: PASS');
