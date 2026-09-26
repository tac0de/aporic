"""Read-only MCP access to the isolated context-treatment database."""
import json, os, subprocess, sys
name=sys.argv[1]
allowed={'aporic_open','aporic_recall','aporic_memory_search'}
if name not in allowed: raise SystemExit('tool not allowed')
workspace='/private/tmp/aporic-core-ab/context/treatment'
p=subprocess.Popen(['/Users/wonyoung_choi/projects/aporic/target/release/aporic','mcp','serve','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,env={**os.environ,'APORIC_DATABASE':'/private/tmp/aporic-core-ab/context/treatment.sqlite3'})
i=0
def call(method,params):
 global i
 i+=1
 p.stdin.write(json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params})+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line: raise RuntimeError(p.stderr.read())
  r=json.loads(line)
  if r.get('id')==i:return r
call('initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'context-pilot','version':'1'}})
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
args={'workspace':workspace}
if name=='aporic_open':args.update({'objective':'Implement merchant settlement summary from current project decisions','idempotency_key':'context-treatment-open'})
if name=='aporic_recall':args.update({'objective':'current merchant settlement row selection, voids, and totals','limit':20})
if name=='aporic_memory_search':args.update({'query':' '.join(sys.argv[2:]) or 'settlement','limit':20})
reply=call('tools/call',{'name':name,'arguments':args})
result=json.loads(reply['result']['content'][0]['text'])
print(json.dumps(result,ensure_ascii=False,indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
