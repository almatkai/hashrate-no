const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
const Ajv=require(path.join(root,'.qa/tooling/node_modules/ajv/dist/2020'));
const addFormats=require(path.join(root,'.qa/tooling/node_modules/ajv-formats'));
const ajv=new Ajv({strict:false,allErrors:true});addFormats(ajv);
const schema=JSON.parse(fs.readFileSync(path.join(root,'.qa/host/schemas/extension-v3.schema.json')));
const validate=ajv.compile(schema),manifest=JSON.parse(fs.readFileSync(path.join(root,'extension.json')));
assert(validate(manifest),JSON.stringify(validate.errors));let positive=1;if(process.argv[2]){const packaged=JSON.parse(fs.readFileSync(process.argv[2]));assert(validate(packaged),JSON.stringify(validate.errors));positive++;}const invalid=[];
for(const key of schema.required){const m=structuredClone(manifest);delete m[key];invalid.push(m);}
for(const [key,value] of [['schemaVersion',2],['id','com.tezbar.hashrate-no'],['execution','in-process'],['protocolVersion',2],['runtime','javascript'],['repository','not a URI'],['commands',[]],['capabilities',['network.anything']],['extra',true],['files',{'../escape':'0'.repeat(64)}],['files',{'bin/tool':'INVALID'}],['artifacts',{'linux':{}}],['artifacts',{'aarch64-apple-darwin':{url:'http://example.test/archive.zip',sha256:'0'.repeat(64),executable:'bin/hashrate-no',executableSha256:'0'.repeat(64)}}]]){const m=structuredClone(manifest);m[key]=value;invalid.push(m);}
for(const m of invalid)assert(!validate(m));
assert.deepEqual(manifest.artifacts,{});assert.deepEqual(manifest.commands[1].arguments,[{name:'power_cost',placeholder:'POWER_COST (0-1.00)',type:'text',required:true}]);
fs.writeFileSync(path.join(root,'evidence/schema-qa.json'),JSON.stringify({schemaCommit:'ea100cee3f69711de12485ba061950c5fc65258f',positive,negative:invalid.length,validator:'AJV 8.17.1 draft2020 + ajv-formats 3.0.1'},null,2)+'\n');
console.log(`Full pinned schema: ${positive} valid manifests, ${invalid.length} negative cases passed.`);
